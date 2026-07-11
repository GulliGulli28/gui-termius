//! Native OS clipboard bridge for CLIPRDR (RDP clipboard redirection) — see
//! CLAUDE.md's "RDP intégré" section for the full design rationale.
//!
//! Windows only for now: `ironrdp-cliprdr-native`'s `WinClipboard` does the
//! actual OS clipboard reads/writes and format negotiation internally (no
//! file-transfer support — `client_capabilities()` on both backends we use
//! advertises none, so the server never offers it); this module's only job
//! is to give `WinClipboard` somewhere to live. It relies on
//! `WM_CLIPBOARDUPDATE` being delivered to a hidden window it owns, which
//! needs an actual Win32 message loop pumping — something this otherwise
//! message-loop-free, pure-tokio process doesn't have without one.
//!
//! On any other platform, `StubClipboard` is a real, complete no-op backend
//! (not a partial implementation) — attaching it still negotiates the
//! CLIPRDR channel so the server doesn't see anything unusual, it just never
//! produces or accepts any clipboard data.

use ironrdp::cliprdr::backend::{CliprdrBackendFactory, ClipboardMessage};
use tokio::sync::mpsc;

/// Sets up the clipboard backend factory (used once per connection attempt
/// to build a fresh `CliprdrBackend`) and the channel `active_session` reads
/// `ClipboardMessage`s from. On Windows, awaits the dedicated clipboard
/// thread finishing its hidden-window setup before returning — a failure
/// there (e.g. window class registration) is reported as an error rather
/// than silently disabling clipboard support.
pub async fn init() -> anyhow::Result<(Box<dyn CliprdrBackendFactory + Send>, mpsc::UnboundedReceiver<ClipboardMessage>)> {
    let (tx, rx) = mpsc::unbounded_channel();

    #[cfg(windows)]
    {
        let factory = windows_impl::spawn(tx).await?;
        Ok((factory, rx))
    }
    #[cfg(not(windows))]
    {
        let _ = tx; // no proxy on this platform: the stub backend never calls it
        Ok((ironrdp_cliprdr_native::StubClipboard::new().backend_factory(), rx))
    }
}

#[cfg(windows)]
mod windows_impl {
    use ironrdp::cliprdr::backend::{CliprdrBackendFactory, ClipboardMessage, ClipboardMessageProxy};
    use tokio::sync::{mpsc, oneshot};
    use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage};

    /// Forwards `ClipboardMessage`s from the clipboard thread into the tokio
    /// world, where `active_session` drives the actual `CliprdrClient` calls
    /// (`initiate_copy`/`submit_format_data`/`initiate_paste`) in response.
    /// `send_clipboard_message` is a plain synchronous call (not `async`),
    /// so this is safe to invoke from a non-tokio OS thread —
    /// `UnboundedSender::send` never blocks or requires an executor.
    #[derive(Debug)]
    struct TokioClipboardProxy {
        tx: mpsc::UnboundedSender<ClipboardMessage>,
    }

    impl ClipboardMessageProxy for TokioClipboardProxy {
        fn send_clipboard_message(&self, message: ClipboardMessage) {
            let _ = self.tx.send(message);
        }
    }

    /// Spawns the dedicated clipboard thread and blocks (asynchronously)
    /// until it's ready. `WinClipboard` is `!Send`: it owns a hidden window
    /// tied to the thread that created it, and that same thread must be the
    /// one pumping Win32 messages for `WM_CLIPBOARDUPDATE` to ever arrive —
    /// so it has to be born, used, and (if this ever returns) dropped
    /// entirely within the spawned thread's closure, never moved across
    /// threads. The thread is intentionally never joined: it keeps pumping
    /// messages for the rest of the process's life, torn down only when the
    /// parent kills us — there is no graceful-shutdown path for this
    /// process (see `rdp-ipc`'s doc comment on why).
    pub(super) async fn spawn(tx: mpsc::UnboundedSender<ClipboardMessage>) -> anyhow::Result<Box<dyn CliprdrBackendFactory + Send>> {
        let (ready_tx, ready_rx) = oneshot::channel();

        std::thread::spawn(move || {
            let proxy = TokioClipboardProxy { tx };
            let win_clipboard = match ironrdp_cliprdr_native::WinClipboard::new(proxy) {
                Ok(wc) => wc,
                Err(e) => {
                    let _ = ready_tx.send(Err(anyhow::anyhow!("initialisation du presse-papiers Windows : {e}")));
                    return;
                }
            };
            let factory = win_clipboard.backend_factory();
            if ready_tx.send(Ok(factory)).is_err() {
                return; // caller gave up waiting — nothing left to serve
            }

            let mut msg = MSG::default();
            loop {
                // SAFETY: `msg` is a valid out-parameter; `hwnd = None` means
                // "any message for a window owned by this thread", which is
                // exactly the hidden window `WinClipboard` just created —
                // including `WM_CLIPRDR_BACKEND_EVENT`, posted internally by
                // `ironrdp-cliprdr-native` to wake this loop up when there's
                // an event to process.
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 <= 0 {
                    break; // WM_QUIT (0) or an error (-1) — stop pumping either way
                }
                // SAFETY: `msg` was just populated by the successful `GetMessageW` call above.
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            // Reached only if something posts WM_QUIT to this thread, which
            // nothing in this codebase does — dropping here would
            // unregister the clipboard listener the same way `Drop` for
            // `WinClipboard` documents.
            drop(win_clipboard);
        });

        match ready_rx.await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("le thread du presse-papiers Windows s'est arrêté avant d'être prêt")),
        }
    }
}
