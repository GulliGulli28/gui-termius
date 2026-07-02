//! SFTP file browsing over an established [`crate::ssh::Connection`].
use crate::ssh::Connection;
use russh_sftp::client::SftpSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
}

pub struct SftpClient {
    session: SftpSession,
}

impl SftpClient {
    pub async fn open(connection: &Connection) -> anyhow::Result<Self> {
        let channel = connection.target().channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let session = SftpSession::new(channel.into_stream()).await?;
        Ok(Self { session })
    }

    pub async fn home_dir(&self) -> anyhow::Result<String> {
        Ok(self.session.canonicalize(".").await?)
    }

    pub async fn list(&self, path: &str) -> anyhow::Result<Vec<Entry>> {
        let mut entries: Vec<Entry> = self
            .session
            .read_dir(path)
            .await?
            .map(|entry| {
                let metadata = entry.metadata();
                Entry {
                    name: entry.file_name(),
                    is_dir: metadata.file_type().is_dir(),
                    is_symlink: metadata.file_type().is_symlink(),
                    size: metadata.len(),
                }
            })
            .collect();
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(entries)
    }

    pub async fn make_dir(&self, path: &str) -> anyhow::Result<()> {
        Ok(self.session.create_dir(path).await?)
    }

    pub async fn remove_file(&self, path: &str) -> anyhow::Result<()> {
        Ok(self.session.remove_file(path).await?)
    }

    pub async fn remove_dir(&self, path: &str) -> anyhow::Result<()> {
        Ok(self.session.remove_dir(path).await?)
    }

    pub async fn rename(&self, from: &str, to: &str) -> anyhow::Result<()> {
        Ok(self.session.rename(from, to).await?)
    }

    pub async fn download(&self, remote_path: &str, local_path: &std::path::Path) -> anyhow::Result<()> {
        let data = self.session.read(remote_path).await?;
        tokio::fs::write(local_path, data).await?;
        Ok(())
    }

    pub async fn upload(&self, local_path: &std::path::Path, remote_path: &str) -> anyhow::Result<()> {
        use tokio::io::AsyncWriteExt;
        let data = tokio::fs::read(local_path).await?;
        // `create` (unlike `write`) opens with CREATE|TRUNCATE, since the
        // remote file generally doesn't exist yet.
        let mut file = self.session.create(remote_path).await?;
        file.write_all(&data).await?;
        Ok(())
    }
}

/// Joins a remote POSIX path with a child segment (`..` navigates up).
pub fn join(base: &str, segment: &str) -> String {
    if segment == ".." {
        let trimmed = base.trim_end_matches('/');
        return match trimmed.rfind('/') {
            Some(0) => "/".to_string(),
            Some(idx) => trimmed[..idx].to_string(),
            None => "/".to_string(),
        };
    }
    if base.ends_with('/') { format!("{base}{segment}") } else { format!("{base}/{segment}") }
}
