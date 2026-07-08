//! Argon2id key derivation + XChaCha20-Poly1305 authenticated encryption — the
//! primitives behind the optional master-password vault ([`crate::master_vault`]).
//!
//! Keys are wrapped in [`SecretKey`], which wipes its bytes on drop.
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroize;

pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 24;
pub const KEY_LEN: usize = 32;

/// Argon2id cost parameters, persisted alongside the vault so a file created
/// with one machine's tuning still opens on another.
#[derive(Debug, Clone, Copy)]
pub struct KdfParams {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        // ~64 MiB, 3 passes: a few hundred ms to unlock on a desktop, but very
        // expensive to brute-force offline.
        Self {
            m_cost: 65536,
            t_cost: 3,
            p_cost: 1,
        }
    }
}

/// A 32-byte key that is zeroized when dropped.
pub struct SecretKey([u8; KEY_LEN]);

impl SecretKey {
    /// A fresh random data key.
    pub fn random() -> Self {
        let mut k = [0u8; KEY_LEN];
        fill_random(&mut k);
        Self(k)
    }

    pub(crate) fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl From<[u8; KEY_LEN]> for SecretKey {
    fn from(b: [u8; KEY_LEN]) -> Self {
        Self(b)
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// A cryptographically-random salt for [`derive_key`].
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut s = [0u8; SALT_LEN];
    fill_random(&mut s);
    s
}

fn fill_random(buf: &mut [u8]) {
    use chacha20poly1305::aead::rand_core::RngCore;
    OsRng.fill_bytes(buf);
}

/// Derives a 32-byte key from `password` and `salt` using Argon2id.
pub fn derive_key(password: &str, salt: &[u8], params: KdfParams) -> anyhow::Result<SecretKey> {
    let p = Params::new(params.m_cost, params.t_cost, params.p_cost, Some(KEY_LEN))
        .map_err(|e| anyhow::anyhow!("paramètres Argon2 invalides : {e}"))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, p);
    let mut key = [0u8; KEY_LEN];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow::anyhow!("échec de la dérivation Argon2 : {e}"))?;
    let out = SecretKey(key);
    key.zeroize();
    Ok(out)
}

/// AEAD-encrypts `plaintext` under `key`, returning `(nonce, ciphertext‖tag)`.
pub fn encrypt(key: &SecretKey, plaintext: &[u8]) -> anyhow::Result<([u8; NONCE_LEN], Vec<u8>)> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| anyhow::anyhow!("échec du chiffrement"))?;
    let mut nonce_arr = [0u8; NONCE_LEN];
    nonce_arr.copy_from_slice(nonce.as_slice());
    Ok((nonce_arr, ct))
}

/// AEAD-decrypts, returning an error if the key is wrong or the bytes were
/// tampered with (the Poly1305 tag won't verify).
pub fn decrypt(
    key: &SecretKey,
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
    let nonce = XNonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("déchiffrement impossible (mot de passe incorrect ou données altérées)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cheap params so the tests don't spend Argon2's full cost.
    fn fast() -> KdfParams {
        KdfParams { m_cost: 512, t_cost: 1, p_cost: 1 }
    }

    #[test]
    fn same_password_and_salt_derive_the_same_key() {
        let salt = random_salt();
        let a = derive_key("correct horse", &salt, fast()).unwrap();
        let b = derive_key("correct horse", &salt, fast()).unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn different_salt_derives_a_different_key() {
        let a = derive_key("pw", &random_salt(), fast()).unwrap();
        let b = derive_key("pw", &random_salt(), fast()).unwrap();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn encrypt_then_decrypt_roundtrips() {
        let key = SecretKey::random();
        let (nonce, ct) = encrypt(&key, b"hunter2").unwrap();
        assert_ne!(ct, b"hunter2", "ciphertext must not be the plaintext");
        assert_eq!(decrypt(&key, &nonce, &ct).unwrap(), b"hunter2");
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let (nonce, ct) = encrypt(&SecretKey::random(), b"secret").unwrap();
        assert!(decrypt(&SecretKey::random(), &nonce, &ct).is_err());
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let key = SecretKey::random();
        let (nonce, mut ct) = encrypt(&key, b"secret").unwrap();
        ct[0] ^= 0x01;
        assert!(
            decrypt(&key, &nonce, &ct).is_err(),
            "a flipped bit must fail the AEAD tag, not decrypt to garbage"
        );
    }
}
