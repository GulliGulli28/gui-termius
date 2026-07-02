//! Glue between `termius_core::ssh` (the SSH transport) and `termius_term`
//! (the transport-agnostic terminal widget): opens a shell on a host and
//! wires its byte streams into a freshly created terminal widget. Also hosts
//! the equivalent glue for SFTP browsing and port forwarding.
use std::sync::{Arc, Mutex};
use termius_core::model::{HostId, PortForward, Workspace};
use termius_core::sftp::{self, SftpClient};
use termius_core::ssh::{self, Connection, ShellInput};
use termius_core::port_forward::{self, ActiveForward};
use tokio::sync::mpsc;

/// Everything kept alive for one open connection tab. `connection` is never
/// read after construction, but it must outlive `terminal`: dropping it
/// would tear down the SSH session (and any bastion hops) the shell runs on.
pub struct TabSession {
    #[allow(dead_code)]
    pub host_id: HostId,
    #[allow(dead_code)]
    pub connection: Connection,
    pub terminal: termius_term::Terminal,
}

pub enum ConnectOutcome {
    Success(Box<TabSession>),
    Failure(String),
}

/// Most of our async results (live SSH sessions, channel handles, ...) own
/// non-`Clone` resources, but `iced` requires application messages to be
/// `Clone` (widgets like buttons clone their stored message). `Arc` is
/// `Clone` regardless of what it wraps, so one-shot async results are
/// smuggled through this single-consume slot instead.
pub struct OneShot<T>(Mutex<Option<T>>);

impl<T> OneShot<T> {
    pub fn new(value: T) -> Arc<Self> {
        Arc::new(Self(Mutex::new(Some(value))))
    }

    pub fn take(&self) -> Option<T> {
        self.0.lock().expect("lock poisoned").take()
    }
}

pub async fn connect_and_build_terminal(workspace: Workspace, host_id: HostId, terminal_id: u64) -> ConnectOutcome {
    match try_connect(&workspace, host_id, terminal_id).await {
        Ok(session) => ConnectOutcome::Success(Box::new(session)),
        Err(err) => ConnectOutcome::Failure(err.to_string()),
    }
}

async fn try_connect(workspace: &Workspace, host_id: HostId, terminal_id: u64) -> anyhow::Result<TabSession> {
    let connection = ssh::connect(workspace, host_id).await?;
    let shell = ssh::open_shell(&connection, 80, 24).await?;
    let terminal = build_terminal(terminal_id, shell)?;
    Ok(TabSession { host_id, connection, terminal })
}

/// Bridges a `ShellSession` (SSH-flavoured input/output) into the
/// transport-agnostic `termius_term::TermCommand` channel the widget expects.
fn build_terminal(id: u64, shell: ssh::ShellSession) -> std::io::Result<termius_term::Terminal> {
    let (term_input_tx, mut term_input_rx) = mpsc::channel::<termius_term::TermCommand>(256);
    let ssh::ShellSession { input: shell_input, output } = shell;

    tokio::spawn(async move {
        while let Some(cmd) = term_input_rx.recv().await {
            let mapped = match cmd {
                termius_term::TermCommand::Write(bytes) => ShellInput::Data(bytes),
                termius_term::TermCommand::Resize { cols, rows } => ShellInput::Resize { cols, rows },
            };
            if shell_input.send(mapped).await.is_err() {
                break;
            }
        }
    });

    let settings = termius_term::settings::Settings {
        font: termius_term::settings::FontSettings::default(),
        theme: termius_term::settings::ThemeSettings::default(),
        backend: termius_term::settings::BackendSettings { input: term_input_tx, output },
    };

    termius_term::Terminal::new(id, settings)
}

/// Which filesystem a [`crate::app::Pane`] is showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneSource {
    Local,
    Remote(HostId),
}

/// Result of opening a pane for the first time: a fresh local listing, or a
/// fresh SSH+SFTP connection plus its home directory's listing.
pub enum PaneOutcome {
    Local { cwd: String, entries: Vec<sftp::Entry> },
    Remote { connection: Connection, client: Arc<SftpClient>, cwd: String, entries: Vec<sftp::Entry> },
    Failure(String),
}

pub async fn open_pane(source: PaneSource, workspace: Workspace) -> PaneOutcome {
    match source {
        PaneSource::Local => {
            let cwd = termius_core::local_fs::home_dir();
            match termius_core::local_fs::list(&cwd) {
                Ok(entries) => PaneOutcome::Local { cwd, entries },
                Err(err) => PaneOutcome::Failure(err.to_string()),
            }
        },
        PaneSource::Remote(host_id) => match try_open_remote_pane(&workspace, host_id).await {
            Ok(outcome) => outcome,
            Err(err) => PaneOutcome::Failure(err.to_string()),
        },
    }
}

async fn try_open_remote_pane(workspace: &Workspace, host_id: HostId) -> anyhow::Result<PaneOutcome> {
    let connection = ssh::connect(workspace, host_id).await?;
    let client = Arc::new(SftpClient::open(&connection).await?);
    let cwd = client.home_dir().await?;
    let entries = client.list(&cwd).await?;
    Ok(PaneOutcome::Remote { connection, client, cwd, entries })
}

/// One-shot result of (re-)listing a directory, threaded back through a `Message`.
pub enum ListOutcome {
    Success { cwd: String, entries: Vec<sftp::Entry> },
    Failure(String),
}

pub async fn list_local(path: String) -> ListOutcome {
    match termius_core::local_fs::list(&path) {
        Ok(entries) => ListOutcome::Success { cwd: path, entries },
        Err(err) => ListOutcome::Failure(err.to_string()),
    }
}

pub async fn list_remote(client: Arc<SftpClient>, path: String) -> ListOutcome {
    match client.list(&path).await {
        Ok(entries) => ListOutcome::Success { cwd: path, entries },
        Err(err) => ListOutcome::Failure(err.to_string()),
    }
}

/// One side of a copy: either the local filesystem, or an already-open SFTP session.
pub enum PaneRef {
    Local,
    Remote(Arc<SftpClient>),
}

pub enum CopyOutcome {
    Success,
    Failure(String),
}

/// Copies `entry` (found in `source_cwd` on `source`) into `dest_cwd` on `dest`.
/// Directories aren't supported yet — only single files.
pub async fn copy_entry(source: PaneRef, source_cwd: String, entry: sftp::Entry, dest: PaneRef, dest_cwd: String) -> CopyOutcome {
    if entry.is_dir {
        return CopyOutcome::Failure("la copie de dossiers n'est pas encore supportée".to_string());
    }
    let result = match (source, dest) {
        (PaneRef::Local, PaneRef::Local) => {
            let src = sftp::join(&source_cwd, &entry.name);
            let dst = sftp::join(&dest_cwd, &entry.name);
            tokio::fs::copy(src, dst).await.map(|_| ()).map_err(anyhow::Error::from)
        },
        (PaneRef::Local, PaneRef::Remote(dst_client)) => {
            let local = std::path::PathBuf::from(sftp::join(&source_cwd, &entry.name));
            let remote = sftp::join(&dest_cwd, &entry.name);
            dst_client.upload(&local, &remote).await
        },
        (PaneRef::Remote(src_client), PaneRef::Local) => {
            let remote = sftp::join(&source_cwd, &entry.name);
            let local = std::path::PathBuf::from(sftp::join(&dest_cwd, &entry.name));
            src_client.download(&remote, &local).await
        },
        (PaneRef::Remote(src_client), PaneRef::Remote(dst_client)) => {
            copy_remote_to_remote(&src_client, &source_cwd, &entry.name, &dst_client, &dest_cwd).await
        },
    };
    match result {
        Ok(()) => CopyOutcome::Success,
        Err(err) => CopyOutcome::Failure(err.to_string()),
    }
}

/// SFTP has no server-to-server copy, so a remote-to-remote transfer is
/// relayed through a temporary local file, same as WinSCP/Termius do.
async fn copy_remote_to_remote(src: &SftpClient, source_cwd: &str, name: &str, dst: &SftpClient, dest_cwd: &str) -> anyhow::Result<()> {
    let tmp = std::env::temp_dir().join(format!("gui-termius-transfer-{}", uuid::Uuid::new_v4()));
    let remote_src = sftp::join(source_cwd, name);
    src.download(&remote_src, &tmp).await?;
    let remote_dst = sftp::join(dest_cwd, name);
    let upload_result = dst.upload(&tmp, &remote_dst).await;
    let _ = tokio::fs::remove_file(&tmp).await;
    upload_result
}

/// A port forward currently relaying traffic, plus the connection it depends on.
pub struct ForwardSession {
    pub connection: Arc<Connection>,
    pub active: ActiveForward,
}

pub enum ForwardOutcome {
    Success(Box<ForwardSession>),
    Failure(String),
}

pub async fn connect_and_start_forward(workspace: Workspace, forward: PortForward) -> ForwardOutcome {
    match try_start_forward(&workspace, forward).await {
        Ok(session) => ForwardOutcome::Success(Box::new(session)),
        Err(err) => ForwardOutcome::Failure(err.to_string()),
    }
}

async fn try_start_forward(workspace: &Workspace, forward: PortForward) -> anyhow::Result<ForwardSession> {
    let connection = Arc::new(ssh::connect(workspace, forward.host_id).await?);
    let active = port_forward::start(connection.clone(), forward).await?;
    Ok(ForwardSession { connection, active })
}

pub async fn stop_forward(session: ForwardSession) {
    session.active.stop(&session.connection).await;
}
