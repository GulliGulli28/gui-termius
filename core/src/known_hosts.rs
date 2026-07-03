//! Trust-on-first-use host key verification, similar in spirit to `~/.ssh/known_hosts`.
//! The first time we see a host's key we record it; on later connections a different
//! key for the same identity is treated as a possible MITM and rejected.
use russh::keys::ssh_key::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    AlreadyTrusted,
    NewlyTrusted,
    Mismatch,
}

#[derive(Default, Serialize, Deserialize)]
struct Store(HashMap<String, String>);

fn path() -> anyhow::Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "gui-termius", "gui-termius")
        .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?;
    Ok(dirs.config_dir().join("known_hosts.json"))
}

fn read() -> Store {
    path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write(store: &Store) -> anyhow::Result<()> {
    let p = path()?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, serde_json::to_string_pretty(store)?)?;
    Ok(())
}

/// Checks `key` against the stored key for `identity` (typically `"host:port"`),
/// trusting it automatically the first time it is seen.
pub fn check_and_trust(identity: &str, key: &PublicKey) -> anyhow::Result<Verdict> {
    let encoded = key.to_openssh()?;
    let mut store = read();
    match store.0.get(identity) {
        Some(existing) if *existing == encoded => Ok(Verdict::AlreadyTrusted),
        Some(_) => Ok(Verdict::Mismatch),
        None => {
            store.0.insert(identity.to_string(), encoded);
            write(&store)?;
            Ok(Verdict::NewlyTrusted)
        },
    }
}

/// Lists every trusted `(identity, openssh-encoded public key)` pair, sorted by identity.
pub fn list() -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = read().0.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

/// Revokes trust for `identity` — the next connection to it will be treated as newly seen.
pub fn remove(identity: &str) -> anyhow::Result<()> {
    let mut store = read();
    store.0.remove(identity);
    write(&store)
}
