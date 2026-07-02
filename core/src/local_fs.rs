//! Local filesystem browsing, sharing the same [`crate::sftp::Entry`] shape
//! as the SFTP client so a local tree and a remote tree can be rendered and
//! navigated identically side by side.
use crate::sftp::Entry;

pub fn home_dir() -> String {
    directories::UserDirs::new().map(|d| d.home_dir().to_string_lossy().to_string()).unwrap_or_else(|| "/".to_string())
}

pub fn list(path: &str) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in std::fs::read_dir(path)? {
        let item = item?;
        let metadata = item.metadata()?;
        entries.push(Entry {
            name: item.file_name().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
            size: metadata.len(),
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}
