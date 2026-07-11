use termius_core::sync_ext::MutexExt;
use crate::state::AppState;
use tauri::State;
use termius_core::model::HostId;
use termius_core::rdp;
use termius_core::vault::{self, SecretKind};

/// Launches the system's RDP client against `host_id` (`HostKind::Rdp`) —
/// see `core::rdp` for why this is a launcher rather than an embedded
/// client. Returns once the client process has started; the session itself
/// is independent of this app from then on.
#[tauri::command]
pub async fn connect_rdp(state: State<'_, AppState>, host_id: HostId) -> Result<(), String> {
    let host = state
        .workspace
        .lock_recover()
        .hosts
        .iter()
        .find(|h| h.id == host_id)
        .cloned()
        .ok_or_else(|| "hôte inconnu".to_string())?;

    let password = vault::load(host_id, SecretKind::Password).map_err(|e| e.to_string())?;

    rdp::launch(&host.address, host.port, &host.username, password.as_deref())
        .await
        .map_err(|e| e.to_string())
}
