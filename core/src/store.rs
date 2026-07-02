//! Persistence of the non-secret workspace (hosts, groups, snippets, tunnels) to disk.
use crate::model::Workspace;
use directories::ProjectDirs;
use std::path::PathBuf;

fn project_dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("dev", "gui-termius", "gui-termius")
        .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))
}

pub fn workspace_path() -> anyhow::Result<PathBuf> {
    let dirs = project_dirs()?;
    Ok(dirs.config_dir().join("workspace.json"))
}

pub fn load() -> anyhow::Result<Workspace> {
    let path = workspace_path()?;
    if !path.exists() {
        return Ok(Workspace::default());
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save(workspace: &Workspace) -> anyhow::Result<()> {
    let path = workspace_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(workspace)?;
    std::fs::write(&path, raw)?;
    Ok(())
}
