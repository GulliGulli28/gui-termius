use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use termius_core::fleet::{self, HostOutcome};
use termius_core::fleet_history::{self, FleetRun};
use termius_core::model::HostId;
use termius_core::sync_ext::MutexExt;

/// Per-host stdout/stderr kept in the persisted history is capped to this many
/// chars — the live UI still gets the full output via the event; only the audit
/// record is trimmed, to keep `fleet_history.json` from ballooning on a verbose
/// command run across many hosts.
const MAX_STORED_OUTPUT: usize = 4000;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FleetOutcomeEvent {
    run_id: String,
    outcome: HostOutcome,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FleetDoneEvent {
    run_id: String,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn truncate(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        return s;
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("\n… (tronqué)");
    out
}

/// Copy of an outcome with its output trimmed for storage (see [`MAX_STORED_OUTPUT`]).
fn for_history(o: &HostOutcome) -> HostOutcome {
    let mut o = o.clone();
    o.stdout = truncate(o.stdout, MAX_STORED_OUTPUT);
    o.stderr = truncate(o.stderr, MAX_STORED_OUTPUT);
    o
}

/// Runs `command` on every host in `host_ids` (SSH only — see
/// [`termius_core::fleet`]), off any PTY, streaming a `fleet-run-outcome` event
/// per host as each finishes, then a single `fleet-run-done`. The completed run
/// is recorded to the persisted history (audit trail). `run_id` is minted by the
/// frontend so several runs can be in flight at once and be told apart on the
/// shared event channel — the command itself resolves only once every host has
/// reported.
#[tauri::command]
pub async fn run_fleet_command(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
    host_ids: Vec<HostId>,
    command: String,
) -> Result<(), String> {
    let started_at_ms = now_ms();
    // Snapshot the workspace so the run sees a consistent view even if the user
    // edits hosts while it's in flight.
    let workspace = Arc::new(state.workspace.lock_recover().clone());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<HostOutcome>();
    // Kept for the recorded run — `command`/`host_ids` are moved into the executor.
    let record_command = command.clone();
    let record_host_ids = host_ids.clone();
    tokio::spawn(fleet::run_on_hosts(
        workspace,
        host_ids,
        command,
        fleet::DEFAULT_CONCURRENCY,
        tx,
    ));

    let mut collected: Vec<HostOutcome> = Vec::new();
    while let Some(outcome) = rx.recv().await {
        collected.push(for_history(&outcome));
        // Emit the full (untruncated) outcome to the live view; ignore emit
        // failures (webview gone) so the run is still fully drained and recorded.
        let _ = app.emit("fleet-run-outcome", FleetOutcomeEvent { run_id: run_id.clone(), outcome });
    }
    let _ = app.emit("fleet-run-done", FleetDoneEvent { run_id });

    let run = FleetRun {
        id: uuid::Uuid::new_v4(),
        started_at_ms,
        command: record_command,
        host_ids: record_host_ids,
        outcomes: collected,
    };
    {
        let mut history = state.fleet_history.lock_recover();
        fleet_history::record(&mut history, run);
        if let Err(e) = fleet_history::save(&history) {
            tracing::warn!("échec de l'enregistrement de l'historique de flotte : {e}");
        }
    }
    Ok(())
}

/// Returns the persisted fleet run history, newest first.
#[tauri::command]
pub fn get_fleet_history(state: State<'_, AppState>) -> Vec<FleetRun> {
    state.fleet_history.lock_recover().clone()
}
