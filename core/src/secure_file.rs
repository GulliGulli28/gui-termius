//! Helpers for writing files that hold credentials or transit sensitive data
//! (workspace, known_hosts, command history, transfer temp files) with
//! owner-only permissions.
//!
//! On Unix these are created `0600` so other local users can't read a
//! private key embedded in `workspace.json`, the trusted host keys, or the
//! plaintext of a file in transit. On non-Unix the OS uses ACLs inherited from
//! the (per-user) parent directory and there is no portable mode, so these fall
//! back to a plain write.
use std::path::Path;

/// Opens `path` for writing (create + truncate) with owner-only permissions
/// (`0600`) on Unix. `.mode()` only takes effect when the file is *created*, so
/// an already-existing file (possibly left `0644` by an older build) has its
/// permissions tightened explicitly.
pub fn open_private(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        let mut perms = file.metadata()?.permissions();
        if perms.mode() & 0o777 != 0o600 {
            perms.set_mode(0o600);
            file.set_permissions(perms)?;
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
    }
}

/// Atomically writes `contents` to `path` with owner-only permissions: the bytes
/// are written to a sibling temp file (0600) which is then renamed over `path`.
///
/// Readers therefore only ever observe the complete old or new file, never a
/// half-written one. That matters because a truncated `known_hosts.json` /
/// `workspace.json` is now treated as corruption (fail-closed) rather than
/// silently ignored: an in-place truncate-then-write could otherwise lose all
/// trusted host keys on a crash mid-write, or be read partially by a concurrent
/// reader (the integration tests share one config dir and run in parallel).
pub fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = temp_sibling(path);
    {
        let mut file = open_private(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// A unique temp path in the same directory as `path` (same filesystem, so the
/// rename is atomic). Unique per call — the pid alone isn't enough since several
/// threads can write the same file at once.
fn temp_sibling(path: &Path) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".tmp.{}.{}", std::process::id(), seq));
    path.with_file_name(name)
}

/// Creates (or tightens to `0600`) an empty file at `path`. Used to pre-create a
/// temp file before handing it to another writer that opens it with `create`
/// (which truncates but preserves an existing file's mode), so the sensitive
/// bytes are never briefly world-readable.
pub fn create_private(path: &Path) -> std::io::Result<()> {
    open_private(path).map(|_| ())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn written_file_is_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.json");
        write_private(&path, b"hunter2").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "file must be readable/writable only by its owner");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hunter2");
    }

    #[test]
    fn tightens_an_existing_loose_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.json");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_private(&path, b"new").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "an existing 0644 file must be tightened to 0600");
    }
}
