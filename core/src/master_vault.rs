//! Optional master-password secret vault: an encrypted file that replaces the
//! OS keychain when the user opts in by setting a master password.
//!
//! Envelope scheme: a random **data key** (DEK) actually encrypts each secret,
//! and the DEK is itself encrypted ("wrapped") under a **key-encryption key**
//! (KEK) derived from the master password via Argon2id. That indirection means
//! changing the master password only re-wraps the DEK — the individual secret
//! entries never have to be re-encrypted. Unwrapping the DEK also doubles as the
//! password check: if the master password is wrong, the AEAD tag on the wrapped
//! DEK fails to verify.
//!
//! The on-disk file is written `0600` (see [`crate::secure_file`]); every field
//! that could reveal a secret is AEAD-encrypted, so the file is safe to sync.
use crate::crypto::{self, KdfParams, SecretKey, NONCE_LEN};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;

/// A base64-encoded `(nonce, ciphertext‖tag)` pair.
#[derive(Serialize, Deserialize, Clone)]
struct Enc {
    nonce: String,
    ct: String,
}

impl Enc {
    fn seal(key: &SecretKey, plaintext: &[u8]) -> anyhow::Result<Self> {
        let (nonce, ct) = crypto::encrypt(key, plaintext)?;
        Ok(Self {
            nonce: B64.encode(nonce),
            ct: B64.encode(ct),
        })
    }

    fn open(&self, key: &SecretKey) -> anyhow::Result<Vec<u8>> {
        let nonce_bytes = B64
            .decode(&self.nonce)
            .map_err(|_| anyhow::anyhow!("nonce illisible dans le coffre"))?;
        let nonce: [u8; NONCE_LEN] = nonce_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("taille de nonce invalide dans le coffre"))?;
        let ct = B64
            .decode(&self.ct)
            .map_err(|_| anyhow::anyhow!("données illisibles dans le coffre"))?;
        crypto::decrypt(key, &nonce, &ct)
    }
}

#[derive(Serialize, Deserialize)]
struct KdfSpec {
    algo: String,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
    salt: String,
}

impl KdfSpec {
    fn new(params: KdfParams, salt: &[u8]) -> Self {
        Self {
            algo: "argon2id".to_string(),
            m_cost: params.m_cost,
            t_cost: params.t_cost,
            p_cost: params.p_cost,
            salt: B64.encode(salt),
        }
    }

    fn params(&self) -> KdfParams {
        KdfParams {
            m_cost: self.m_cost,
            t_cost: self.t_cost,
            p_cost: self.p_cost,
        }
    }

    fn salt_bytes(&self) -> anyhow::Result<Vec<u8>> {
        B64.decode(&self.salt)
            .map_err(|_| anyhow::anyhow!("sel illisible dans le coffre"))
    }
}

#[derive(Serialize, Deserialize)]
struct VaultDoc {
    version: u32,
    kdf: KdfSpec,
    /// The DEK, encrypted under the KEK derived from the master password.
    wrapped_dek: Enc,
    /// Secret entries, each encrypted under the DEK, keyed by `"<id>:<kind>"`.
    entries: BTreeMap<String, Enc>,
}

/// An unlocked, in-memory view of the vault, holding the decrypted data key.
/// Dropping it wipes the key (the DEK is a [`SecretKey`]).
pub struct MasterVault {
    path: PathBuf,
    dek: SecretKey,
    doc: VaultDoc,
}

impl MasterVault {
    /// Whether an encrypted vault file already exists at `path`.
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    /// Creates a brand-new, empty vault protected by `password`.
    pub fn create(path: &Path, password: &str) -> anyhow::Result<Self> {
        let params = KdfParams::default();
        let salt = crypto::random_salt();
        let kek = crypto::derive_key(password, &salt, params)?;
        let dek = SecretKey::random();
        let wrapped_dek = Enc::seal(&kek, dek.as_bytes())?;
        let doc = VaultDoc {
            version: VERSION,
            kdf: KdfSpec::new(params, &salt),
            wrapped_dek,
            entries: BTreeMap::new(),
        };
        let vault = Self {
            path: path.to_path_buf(),
            dek,
            doc,
        };
        vault.persist()?;
        Ok(vault)
    }

    /// Opens an existing vault. Returns an error if `password` is wrong (the
    /// wrapped-DEK tag fails to verify).
    pub fn unlock(path: &Path, password: &str) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let doc: VaultDoc = serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("fichier de coffre corrompu : {e}"))?;
        if doc.version != VERSION {
            anyhow::bail!("version de coffre non prise en charge : {}", doc.version);
        }
        let kek = crypto::derive_key(password, &doc.kdf.salt_bytes()?, doc.kdf.params())?;
        let dek_bytes = doc
            .wrapped_dek
            .open(&kek)
            .map_err(|_| anyhow::anyhow!("mot de passe maître incorrect"))?;
        let dek_arr: [u8; crypto::KEY_LEN] = dek_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("clé de données de taille invalide"))?;
        Ok(Self {
            path: path.to_path_buf(),
            dek: SecretKey::from(dek_arr),
            doc,
        })
    }

    /// Decrypts the secret stored under `key`, if any.
    pub fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        match self.doc.entries.get(key) {
            None => Ok(None),
            Some(entry) => {
                let bytes = entry.open(&self.dek)?;
                let s = String::from_utf8(bytes)
                    .map_err(|_| anyhow::anyhow!("secret non-UTF-8 dans le coffre"))?;
                Ok(Some(s))
            }
        }
    }

    /// Encrypts `value` under `key` and persists the vault.
    pub fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let entry = Enc::seal(&self.dek, value.as_bytes())?;
        self.doc.entries.insert(key.to_string(), entry);
        self.persist()
    }

    /// Removes the secret stored under `key` (if present) and persists.
    pub fn remove(&mut self, key: &str) -> anyhow::Result<()> {
        if self.doc.entries.remove(key).is_some() {
            self.persist()?;
        }
        Ok(())
    }

    /// Re-derives the KEK from `new_password` (with a fresh salt) and re-wraps
    /// the existing DEK. Entries are untouched, so this is cheap.
    pub fn change_password(&mut self, new_password: &str) -> anyhow::Result<()> {
        let params = KdfParams::default();
        let salt = crypto::random_salt();
        let kek = crypto::derive_key(new_password, &salt, params)?;
        self.doc.wrapped_dek = Enc::seal(&kek, self.dek.as_bytes())?;
        self.doc.kdf = KdfSpec::new(params, &salt);
        self.persist()
    }

    fn persist(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(&self.doc)?;
        crate::secure_file::write_private(&self.path, raw.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Override the cost so tests aren't slow. `create`/`change_password` use the
    // default params internally; that's fine — the default is still only a few
    // hundred ms and these tests each derive at most a couple of keys.
    fn vault_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("secrets.enc")
    }

    #[test]
    fn set_get_roundtrips_and_survives_relock() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(&dir);

        {
            let mut v = MasterVault::create(&path, "master-pw").unwrap();
            v.set("host-1:password", "hunter2").unwrap();
            v.set("key-9:passphrase", "corrèze").unwrap();
        } // dropped: DEK wiped, only the encrypted file remains

        let v = MasterVault::unlock(&path, "master-pw").unwrap();
        assert_eq!(v.get("host-1:password").unwrap().as_deref(), Some("hunter2"));
        assert_eq!(v.get("key-9:passphrase").unwrap().as_deref(), Some("corrèze"));
        assert_eq!(v.get("missing").unwrap(), None);
    }

    #[test]
    fn wrong_master_password_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(&dir);
        MasterVault::create(&path, "right").unwrap();
        assert!(MasterVault::unlock(&path, "wrong").is_err());
    }

    #[test]
    fn on_disk_file_never_contains_the_plaintext_secret() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(&dir);
        let mut v = MasterVault::create(&path, "pw").unwrap();
        v.set("h:password", "SUPERSECRET").unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("SUPERSECRET"), "secret leaked in cleartext");
    }

    #[test]
    fn tampering_with_an_entry_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(&dir);
        {
            let mut v = MasterVault::create(&path, "pw").unwrap();
            v.set("h:password", "secret").unwrap();
        }
        // Corrupt one ciphertext byte in the stored entry.
        let mut doc: VaultDoc =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = doc.entries.get_mut("h:password").unwrap();
        let mut ct = B64.decode(&entry.ct).unwrap();
        ct[0] ^= 0x01;
        entry.ct = B64.encode(&ct);
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();

        let v = MasterVault::unlock(&path, "pw").unwrap();
        assert!(v.get("h:password").is_err(), "tampered entry must not decrypt");
    }

    #[test]
    fn change_password_keeps_entries_and_swaps_the_password() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(&dir);
        {
            let mut v = MasterVault::create(&path, "old-pw").unwrap();
            v.set("h:password", "keepme").unwrap();
            v.change_password("new-pw").unwrap();
        }
        assert!(MasterVault::unlock(&path, "old-pw").is_err(), "old password must stop working");
        let v = MasterVault::unlock(&path, "new-pw").unwrap();
        assert_eq!(v.get("h:password").unwrap().as_deref(), Some("keepme"));
    }
}
