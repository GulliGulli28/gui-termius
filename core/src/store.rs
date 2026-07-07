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
    load_from(&workspace_path()?)
}

pub fn save(workspace: &Workspace) -> anyhow::Result<()> {
    save_to(&workspace_path()?, workspace)
}

fn load_from(path: &PathBuf) -> anyhow::Result<Workspace> {
    if !path.exists() {
        return Ok(Workspace::default());
    }
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn save_to(path: &PathBuf, workspace: &Workspace) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(workspace)?;
    std::fs::write(path, raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Group, Host, Workspace};

    fn sample_workspace() -> Workspace {
        let mut ws = Workspace::default();
        ws.groups.push(Group { id: uuid::Uuid::new_v4(), name: "prod".into(), parent_id: None, icon: None, color: None });
        ws.hosts.push(Host::new("box", "example.com", "root"));
        ws
    }

    #[test]
    fn load_missing_file_returns_default_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.json");

        let ws = load_from(&path).unwrap();

        assert!(ws.hosts.is_empty());
        assert!(ws.groups.is_empty());
    }

    #[test]
    fn save_then_load_roundtrips_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.json");
        let original = sample_workspace();

        save_to(&path, &original).unwrap();
        let reloaded = load_from(&path).unwrap();

        assert_eq!(reloaded.hosts.len(), 1);
        assert_eq!(reloaded.hosts[0].label, "box");
        assert_eq!(reloaded.groups.len(), 1);
        assert_eq!(reloaded.groups[0].name, "prod");
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("workspace.json");

        save_to(&path, &Workspace::default()).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn load_rejects_corrupted_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.json");
        std::fs::write(&path, "{ not valid json").unwrap();

        assert!(load_from(&path).is_err());
    }

    #[test]
    fn load_tolerates_old_workspace_missing_new_fields() {
        // Old workspace.json files predate the `keychain`/`custom_icons` fields;
        // both are `#[serde(default)]` so loading them must not fail.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.json");
        std::fs::write(&path, r#"{"groups":[],"hosts":[],"snippets":[],"portForwards":[]}"#).unwrap();

        let ws = load_from(&path).unwrap();

        assert!(ws.keychain.is_empty());
        assert!(ws.custom_icons.is_empty());
    }
}
