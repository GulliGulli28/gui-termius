//! Structured fan-out execution: run one command across many hosts at once and
//! capture a per-host result (exit code, stdout, stderr, duration) instead of
//! streaming bytes into an interactive terminal. This is the "control plane"
//! primitive the terminal path deliberately lacks — and the foundation the
//! later facts-collection and declarative-intent layers build on.
//!
//! MVP scope: SSH hosts only. A `dockerExec` host is a daemon, not a single
//! container, so a Docker fleet run would need a per-target container choice;
//! RDP has no shell; K8s exec has no backend yet. Those are reported as a
//! per-host error rather than silently skipped, and the UI only offers SSH
//! hosts as targets.

use crate::model::{HostId, HostKind, Workspace};
use crate::ssh;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Semaphore};

/// Default number of hosts a single fleet run connects to concurrently.
pub const DEFAULT_CONCURRENCY: usize = 10;

/// Result of running a command on one host in a fleet run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostOutcome {
    pub host_id: HostId,
    /// Exit code when the command actually ran to completion; `None` means it
    /// never ran — see `error`. `Some(0)` with `error: None` is the success case.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    /// Set when the host couldn't be reached / the command couldn't be started
    /// at all (connection, auth, unsupported host kind…), as distinct from a
    /// command that ran and returned a non-zero `exit_code`.
    pub error: Option<String>,
}

/// Runs `command` on every host in `host_ids` concurrently — bounded by
/// `concurrency` — sending each [`HostOutcome`] on `tx` as soon as that host
/// finishes. Returns once every host has reported; dropping the last `tx` clone
/// (which happens when this returns) lets the receiver observe completion.
pub async fn run_on_hosts(
    workspace: Arc<Workspace>,
    host_ids: Vec<HostId>,
    command: String,
    concurrency: usize,
    tx: mpsc::UnboundedSender<HostOutcome>,
) {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut handles = Vec::with_capacity(host_ids.len());
    for host_id in host_ids {
        let workspace = workspace.clone();
        let command = command.clone();
        let semaphore = semaphore.clone();
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            // Held for the whole per-host run so no more than `concurrency`
            // hosts are connected at once; released when this task ends.
            let _permit = semaphore.acquire().await;
            let outcome = run_one(&workspace, host_id, &command).await;
            let _ = tx.send(outcome);
        }));
    }
    for handle in handles {
        let _ = handle.await;
    }
}

async fn run_one(workspace: &Workspace, host_id: HostId, command: &str) -> HostOutcome {
    let started = Instant::now();
    let result = execute(workspace, host_id, command).await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(output) => HostOutcome {
            host_id,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms,
            error: None,
        },
        Err(e) => HostOutcome {
            host_id,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms,
            error: Some(e.to_string()),
        },
    }
}

async fn execute(
    workspace: &Workspace,
    host_id: HostId,
    command: &str,
) -> anyhow::Result<ssh::CommandOutput> {
    let host = workspace
        .host(host_id)
        .ok_or_else(|| anyhow::anyhow!("hôte inconnu"))?;
    match host.kind {
        HostKind::Ssh => {
            let mut connection = ssh::connect(workspace, host_id).await?;
            let output = ssh::run_command_capture(&connection, command).await;
            connection.disconnect().await;
            output
        }
        HostKind::DockerExec => {
            anyhow::bail!("Docker exec pas encore supporté en flotte (nécessite un choix de conteneur)")
        }
        HostKind::Rdp => anyhow::bail!("RDP n'a pas de shell — non exécutable en flotte"),
        HostKind::K8sExec => anyhow::bail!("K8s exec sans backend — non exécutable en flotte"),
    }
}
