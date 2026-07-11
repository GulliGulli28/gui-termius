use crate::state::{AppState, RdpViewSession};
use crate::util;
use rdp_ipc::{ClientMessage, ConnectRequest, SidecarMessage};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use termius_core::model::HostId;
use termius_core::sync_ext::MutexExt;
use termius_core::vault::{self, SecretKind};
use tokio::io::AsyncWriteExt as _;
use uuid::Uuid;

/// `pixels`: base64-encoded RGBA8, row-major, `4 * width * height` bytes —
/// same encoding convention as `terminal-data` (see `commands::terminal`).
#[derive(Serialize, Clone)]
struct RdpViewFrameEvent {
    id: String,
    width: u16,
    height: u16,
    pixels: String,
}

#[derive(Serialize, Clone)]
struct RdpViewErrorEvent {
    id: String,
    message: String,
}

#[derive(Serialize, Clone)]
struct RdpViewClosedEvent {
    id: String,
}

/// Starts an embedded, view-only RDP session against `host_id`
/// (`HostKind::Rdp`) by spawning the `rdp-sidecar` process and streaming its
/// decoded framebuffer back as `rdp-view-frame`/`rdp-view-error`/
/// `rdp-view-closed` events, all carrying this call's returned session id.
///
/// See `core::rdp` for the older launcher-mode alternative (still used by
/// `connect_rdp`) and CLAUDE.md's "Pourquoi un processus RDP séparé" section
/// for why this runs in a separate process instead of linking IronRDP
/// directly into this binary.
///
/// Mouse/keyboard input is forwarded separately, see `send_rdp_view_input`.
/// `width`/`height`: the view's container size at connect time (measured by
/// `RdpTab.tsx`), used as the session's initial resolution instead of an
/// arbitrary fixed default — the sidecar clamps them to MS-RDPEDISP's valid
/// range regardless.
#[tauri::command]
pub async fn connect_rdp_view(
    app: AppHandle,
    state: State<'_, AppState>,
    host_id: HostId,
    width: u16,
    height: u16,
) -> Result<String, String> {
    let host = state
        .workspace
        .lock_recover()
        .hosts
        .iter()
        .find(|h| h.id == host_id)
        .cloned()
        .ok_or_else(|| "hôte inconnu".to_string())?;

    let password = vault::load(host_id, SecretKind::Password)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "aucun mot de passe enregistré pour cet hôte".to_string())?;

    let request = ConnectRequest { host: host.address.clone(), port: host.port, username: host.username.clone(), password, width, height };

    let (mut events, child) = app
        .shell()
        .sidecar("rdp-sidecar")
        .map_err(|e| format!("rdp-sidecar introuvable : {e}"))?
        .set_raw_out(true)
        .spawn()
        .map_err(|e| format!("échec du lancement de rdp-sidecar : {e}"))?;

    let session_id = Uuid::new_v4().to_string();

    let mut line = serde_json::to_vec(&request).expect("ConnectRequest always serializes");
    line.push(b'\n');
    let mut child = child;
    child.write(&line).map_err(|e| format!("écriture vers rdp-sidecar : {e}"))?;

    state.rdp_views.lock_recover().insert(session_id.clone(), RdpViewSession { child });

    let (mut duplex_read, mut duplex_write) = tokio::io::duplex(256 * 1024);
    let crash_app = app.clone();
    let crash_id = session_id.clone();
    tokio::spawn(async move {
        // In a release build the app has no console (`windows_subsystem =
        // "windows"` in main.rs), so `tracing::warn!`/`tracing::error!` below
        // land nowhere the user can see — a real panic's message (printed by
        // Rust's default panic hook to the sidecar's stderr) would otherwise
        // vanish entirely, leaving only a generic "session closed" once
        // `duplex_write` drops. Keep a short tail of stderr so an abnormal
        // exit can carry it into an `rdp-view-error` the UI actually shows.
        const STDERR_TAIL_MAX: usize = 4096;
        let mut stderr_tail: Vec<u8> = Vec::new();
        while let Some(event) = events.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    if duplex_write.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                CommandEvent::Stderr(bytes) => {
                    tracing::warn!(stderr = %String::from_utf8_lossy(&bytes), "rdp-sidecar");
                    stderr_tail.extend_from_slice(&bytes);
                    let excess = stderr_tail.len().saturating_sub(STDERR_TAIL_MAX);
                    if excess > 0 {
                        stderr_tail.drain(0..excess);
                    }
                }
                CommandEvent::Error(e) => {
                    tracing::error!(error = %e, "rdp-sidecar pipe error");
                    let _ = crash_app.emit(
                        "rdp-view-error",
                        RdpViewErrorEvent { id: crash_id.clone(), message: format!("erreur de communication avec rdp-sidecar : {e}") },
                    );
                    break;
                }
                CommandEvent::Terminated(payload) => {
                    // A graceful exit (whether the session ended cleanly or
                    // with a handled error) always writes its own
                    // `SidecarMessage` to stdout before the process returns
                    // `()` from `main`, which exits 0 — see `main.rs`. A
                    // non-zero/signalled exit here means the process crashed
                    // (e.g. panicked) without going through that path.
                    if payload.code != Some(0) {
                        let detail = String::from_utf8_lossy(&stderr_tail).trim().to_string();
                        let message = if detail.is_empty() {
                            format!("rdp-sidecar s'est arrêté de façon inattendue (code {:?}, signal {:?})", payload.code, payload.signal)
                        } else {
                            format!("rdp-sidecar a planté : {detail}")
                        };
                        let _ = crash_app.emit("rdp-view-error", RdpViewErrorEvent { id: crash_id.clone(), message });
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    let bridge_id = session_id.clone();
    let bridge_app = app.clone();
    tokio::spawn(async move {
        loop {
            let message = match SidecarMessage::read_from(&mut duplex_read).await {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    let _ = bridge_app.emit("rdp-view-closed", RdpViewClosedEvent { id: bridge_id.clone() });
                    break;
                }
                Err(e) => {
                    let _ = bridge_app.emit("rdp-view-error", RdpViewErrorEvent { id: bridge_id.clone(), message: e.to_string() });
                    break;
                }
            };
            match message {
                SidecarMessage::Image { width, height, pixels } => {
                    let payload = RdpViewFrameEvent { id: bridge_id.clone(), width, height, pixels: util::encode(&pixels) };
                    if bridge_app.emit("rdp-view-frame", payload).is_err() {
                        break;
                    }
                }
                SidecarMessage::Error(msg) => {
                    let _ = bridge_app.emit("rdp-view-error", RdpViewErrorEvent { id: bridge_id.clone(), message: msg });
                    break;
                }
                SidecarMessage::Closed => {
                    let _ = bridge_app.emit("rdp-view-closed", RdpViewClosedEvent { id: bridge_id.clone() });
                    break;
                }
            }
        }
    });

    Ok(session_id)
}

/// Forwards one mouse/keyboard event to an already-connected session's
/// `rdp-sidecar` stdin. `message` is decoded straight from the frontend's
/// `invoke()` call — see `rdp_ipc::ClientMessage` for the wire shape and
/// `rdp-sidecar/src/input.rs` for how it becomes an RDP scancode/PDU.
#[tauri::command]
pub fn send_rdp_view_input(state: State<'_, AppState>, session_id: String, message: ClientMessage) -> Result<(), String> {
    let mut sessions = state.rdp_views.lock_recover();
    let session = sessions.get_mut(&session_id).ok_or_else(|| "session inconnue".to_string())?;
    let mut line = serde_json::to_vec(&message).expect("ClientMessage always serializes");
    line.push(b'\n');
    session.child.write(&line).map_err(|e| format!("écriture vers rdp-sidecar : {e}"))
}

#[tauri::command]
pub fn close_rdp_view(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    if let Some(session) = state.rdp_views.lock_recover().remove(&session_id) {
        // Best-effort: the sidecar also exits on its own once its stdout
        // pipe breaks (main.rs's write loop hits a closed reader), so a
        // failed kill here (already-dead process) isn't an error.
        let _ = session.child.kill();
    }
    Ok(())
}
