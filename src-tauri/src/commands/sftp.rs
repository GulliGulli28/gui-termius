use crate::state::{AppState, Pane};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use termius_core::model::HostId;
use termius_core::sftp::{Entry, SftpClient};
use termius_core::ssh;
use termius_core::transfer::{self, PaneRef};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PaneSource {
    Local,
    Remote {
        #[serde(rename = "hostId")]
        host_id: HostId,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneOpened {
    pub pane_id: String,
    pub cwd: String,
    pub entries: Vec<Entry>,
}

#[tauri::command]
pub async fn open_pane(state: State<'_, AppState>, source: PaneSource) -> Result<PaneOpened, String> {
    let pane_id = Uuid::new_v4().to_string();
    match source {
        PaneSource::Local => {
            let cwd = termius_core::local_fs::home_dir();
            let entries = termius_core::local_fs::list(&cwd).map_err(|e| e.to_string())?;
            state.panes.lock().expect("lock poisoned").insert(pane_id.clone(), Pane { connection: None, client: None });
            Ok(PaneOpened { pane_id, cwd, entries })
        },
        PaneSource::Remote { host_id } => {
            let workspace = state.workspace.lock().expect("lock poisoned").clone();
            let connection = ssh::connect(&workspace, host_id).await.map_err(|e| e.to_string())?;
            let client = Arc::new(SftpClient::open(&connection).await.map_err(|e| e.to_string())?);
            let cwd = client.home_dir().await.map_err(|e| e.to_string())?;
            let entries = client.list(&cwd).await.map_err(|e| e.to_string())?;
            state.panes.lock().expect("lock poisoned").insert(pane_id.clone(), Pane { connection: Some(connection), client: Some(client) });
            Ok(PaneOpened { pane_id, cwd, entries })
        },
    }
}

#[tauri::command]
pub fn close_pane(state: State<'_, AppState>, pane_id: String) -> Result<(), String> {
    state.panes.lock().expect("lock poisoned").remove(&pane_id);
    Ok(())
}

#[derive(Serialize)]
pub struct PaneListed {
    pub cwd: String,
    pub entries: Vec<Entry>,
}

fn pane_ref(state: &AppState, pane_id: &str) -> Result<PaneRef, String> {
    let panes = state.panes.lock().expect("lock poisoned");
    let pane = panes.get(pane_id).ok_or_else(|| "pane inconnu".to_string())?;
    Ok(match &pane.client {
        Some(client) => PaneRef::Remote(client.clone()),
        None => PaneRef::Local,
    })
}

#[tauri::command]
pub async fn list_pane(state: State<'_, AppState>, pane_id: String, path: String) -> Result<PaneListed, String> {
    let reference = pane_ref(&state, &pane_id)?;
    let entries = transfer::list(&reference, &path).await.map_err(|e| e.to_string())?;
    Ok(PaneListed { cwd: path, entries })
}

#[tauri::command]
pub async fn copy_entry(
    state: State<'_, AppState>,
    source_pane_id: String,
    source_cwd: String,
    entry: Entry,
    dest_pane_id: String,
    dest_cwd: String,
) -> Result<PaneListed, String> {
    let source = pane_ref(&state, &source_pane_id)?;
    let dest = pane_ref(&state, &dest_pane_id)?;
    transfer::copy_entry(&source, &source_cwd, &entry, &dest, &dest_cwd).await.map_err(|e| e.to_string())?;
    let entries = transfer::list(&dest, &dest_cwd).await.map_err(|e| e.to_string())?;
    Ok(PaneListed { cwd: dest_cwd, entries })
}
