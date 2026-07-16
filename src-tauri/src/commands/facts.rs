use crate::state::AppState;
use std::sync::Arc;
use tauri::State;
use termius_core::facts::{self, FactsOutcome};
use termius_core::fleet;
use termius_core::model::HostId;
use termius_core::sync_ext::MutexExt;

/// Collects live state (OS, kernel, CPU, load, memory) for every host in
/// `host_ids` concurrently (SSH only — see [`termius_core::facts`]). Batch:
/// resolves once every host has reported, returning one [`FactsOutcome`] each.
#[tauri::command]
pub async fn collect_facts(
    state: State<'_, AppState>,
    host_ids: Vec<HostId>,
) -> Result<Vec<FactsOutcome>, String> {
    let workspace = Arc::new(state.workspace.lock_recover().clone());
    Ok(facts::collect(workspace, host_ids, fleet::DEFAULT_CONCURRENCY).await)
}
