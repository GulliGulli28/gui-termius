//! Generic local/remote file transfer helpers shared by any UI: copying
//! between two arbitrary "panes" (local filesystem or an open SFTP session),
//! including the remote-to-remote case which SFTP can't do directly.
use crate::local_fs;
use crate::sftp::{self, Entry, SftpClient};
use std::sync::Arc;

pub enum PaneRef {
    Local,
    Remote(Arc<SftpClient>),
}

pub async fn list(pane: &PaneRef, path: &str) -> anyhow::Result<Vec<Entry>> {
    match pane {
        PaneRef::Local => local_fs::list(path),
        PaneRef::Remote(client) => client.list(path).await,
    }
}

/// Copies `entry` (file or directory) from `source_cwd` on `source` into `dest_cwd` on `dest`.
/// Directories are copied recursively.
pub async fn copy_entry(source: &PaneRef, source_cwd: &str, entry: &Entry, dest: &PaneRef, dest_cwd: &str) -> anyhow::Result<()> {
    if entry.is_dir {
        return copy_dir(source, source_cwd, &entry.name, dest, dest_cwd).await;
    }
    match (source, dest) {
        (PaneRef::Local, PaneRef::Local) => {
            let src = sftp::join(source_cwd, &entry.name);
            let dst = sftp::join(dest_cwd, &entry.name);
            tokio::fs::copy(src, dst).await?;
            Ok(())
        },
        (PaneRef::Local, PaneRef::Remote(dst_client)) => {
            let local = std::path::PathBuf::from(sftp::join(source_cwd, &entry.name));
            let remote = sftp::join(dest_cwd, &entry.name);
            dst_client.upload(&local, &remote).await
        },
        (PaneRef::Remote(src_client), PaneRef::Local) => {
            let remote = sftp::join(source_cwd, &entry.name);
            let local = std::path::PathBuf::from(sftp::join(dest_cwd, &entry.name));
            src_client.download(&remote, &local).await
        },
        (PaneRef::Remote(src_client), PaneRef::Remote(dst_client)) => {
            copy_remote_to_remote_file(src_client, source_cwd, &entry.name, dst_client, dest_cwd).await
        },
    }
}

fn copy_dir<'a>(
    source: &'a PaneRef,
    source_dir: &'a str,
    name: &'a str,
    dest: &'a PaneRef,
    dest_dir: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let src_path = sftp::join(source_dir, name);
        let dst_path = sftp::join(dest_dir, name);

        // Create destination directory
        match dest {
            PaneRef::Local => {
                tokio::fs::create_dir_all(&dst_path).await?;
            }
            PaneRef::Remote(client) => {
                client.make_dir(&dst_path).await?;
            }
        }

        // List source directory
        let entries = match source {
            PaneRef::Local => local_fs::list(&src_path)?,
            PaneRef::Remote(client) => client.list(&src_path).await?,
        };

        for child in entries {
            if child.is_dir {
                copy_dir(source, &src_path, &child.name, dest, &dst_path).await?;
            } else {
                match (source, dest) {
                    (PaneRef::Local, PaneRef::Local) => {
                        let s = sftp::join(&src_path, &child.name);
                        let d = sftp::join(&dst_path, &child.name);
                        tokio::fs::copy(s, d).await?;
                    }
                    (PaneRef::Local, PaneRef::Remote(dc)) => {
                        let local = std::path::PathBuf::from(sftp::join(&src_path, &child.name));
                        let remote = sftp::join(&dst_path, &child.name);
                        dc.upload(&local, &remote).await?;
                    }
                    (PaneRef::Remote(sc), PaneRef::Local) => {
                        let remote = sftp::join(&src_path, &child.name);
                        let local = std::path::PathBuf::from(sftp::join(&dst_path, &child.name));
                        sc.download(&remote, &local).await?;
                    }
                    (PaneRef::Remote(sc), PaneRef::Remote(dc)) => {
                        copy_remote_to_remote_file(sc, &src_path, &child.name, dc, &dst_path).await?;
                    }
                }
            }
        }
        Ok(())
    })
}

/// SFTP has no server-to-server copy, so a remote-to-remote transfer is
/// relayed through a temporary local file, same as WinSCP/Termius do.
async fn copy_remote_to_remote_file(src: &SftpClient, source_cwd: &str, name: &str, dst: &SftpClient, dest_cwd: &str) -> anyhow::Result<()> {
    let tmp = std::env::temp_dir().join(format!("gui-termius-transfer-{}", uuid::Uuid::new_v4()));
    let remote_src = sftp::join(source_cwd, name);
    src.download(&remote_src, &tmp).await?;
    let remote_dst = sftp::join(dest_cwd, name);
    let upload_result = dst.upload(&tmp, &remote_dst).await;
    let _ = tokio::fs::remove_file(&tmp).await;
    upload_result
}
