//! Secret storage backed by the OS keychain (Keychain/Credential Manager/Secret Service).
//! When the OS keychain is unavailable (e.g. headless Linux or WSL without Secret Service),
//! secrets fall back to a process-lifetime in-memory store so password auth still works
//! for the duration of the session. On restart the user must re-enter the password once.
use crate::model::HostId;
use keyring::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "gui-termius";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Password,
    KeyPassphrase,
}

impl SecretKind {
    fn suffix(self) -> &'static str {
        match self {
            SecretKind::Password => "password",
            SecretKind::KeyPassphrase => "passphrase",
        }
    }
}

fn entry(host_id: HostId, kind: SecretKind) -> Result<Entry, keyring::Error> {
    Entry::new(SERVICE, &format!("{host_id}:{}", kind.suffix()))
}

fn fallback_key(host_id: HostId, kind: SecretKind) -> String {
    format!("{host_id}:{}", kind.suffix())
}

fn fallback() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn store(host_id: HostId, kind: SecretKind, secret: &str) -> anyhow::Result<()> {
    let keychain_ok = entry(host_id, kind)
        .and_then(|e| e.set_password(secret))
        .is_ok();
    if !keychain_ok {
        tracing::debug!("OS keychain unavailable, storing secret for {host_id} in process memory");
        fallback()
            .lock()
            .unwrap()
            .insert(fallback_key(host_id, kind), secret.to_owned());
    }
    Ok(())
}

pub fn load(host_id: HostId, kind: SecretKind) -> anyhow::Result<Option<String>> {
    match entry(host_id, kind).and_then(|e| e.get_password()) {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(fallback()
            .lock()
            .unwrap()
            .get(&fallback_key(host_id, kind))
            .cloned()),
        Err(err) => {
            tracing::debug!("OS keychain unavailable for load ({err}), checking process memory");
            Ok(fallback()
                .lock()
                .unwrap()
                .get(&fallback_key(host_id, kind))
                .cloned())
        }
    }
}

pub fn delete(host_id: HostId, kind: SecretKind) -> anyhow::Result<()> {
    fallback()
        .lock()
        .unwrap()
        .remove(&fallback_key(host_id, kind));
    match entry(host_id, kind) {
        Ok(e) => match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        },
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // Each test uses its own random host_id so runs never collide with each
    // other or with real entries, and always deletes what it stored: on
    // machines with a working OS keychain these round-trip through the real
    // Credential Manager / Keychain / Secret Service, not just the fallback map.

    #[test]
    fn load_missing_secret_returns_none() {
        let host_id = Uuid::new_v4();
        assert!(load(host_id, SecretKind::Password).unwrap().is_none());
    }

    #[test]
    fn store_then_load_roundtrips() {
        let host_id = Uuid::new_v4();
        store(host_id, SecretKind::Password, "hunter2").unwrap();

        assert_eq!(load(host_id, SecretKind::Password).unwrap().as_deref(), Some("hunter2"));

        delete(host_id, SecretKind::Password).unwrap();
        assert!(load(host_id, SecretKind::Password).unwrap().is_none());
    }

    #[test]
    fn password_and_passphrase_are_independent() {
        let host_id = Uuid::new_v4();
        store(host_id, SecretKind::Password, "pw-secret").unwrap();
        store(host_id, SecretKind::KeyPassphrase, "passphrase-secret").unwrap();

        assert_eq!(load(host_id, SecretKind::Password).unwrap().as_deref(), Some("pw-secret"));
        assert_eq!(load(host_id, SecretKind::KeyPassphrase).unwrap().as_deref(), Some("passphrase-secret"));

        delete(host_id, SecretKind::Password).unwrap();
        delete(host_id, SecretKind::KeyPassphrase).unwrap();
    }

    #[test]
    fn delete_is_idempotent_for_missing_entry() {
        let host_id = Uuid::new_v4();
        assert!(delete(host_id, SecretKind::Password).is_ok());
        assert!(delete(host_id, SecretKind::Password).is_ok());
    }

    #[test]
    fn overwriting_a_secret_replaces_it() {
        let host_id = Uuid::new_v4();
        store(host_id, SecretKind::Password, "first").unwrap();
        store(host_id, SecretKind::Password, "second").unwrap();

        assert_eq!(load(host_id, SecretKind::Password).unwrap().as_deref(), Some("second"));

        delete(host_id, SecretKind::Password).unwrap();
    }
}
