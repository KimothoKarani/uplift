use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Cipher(Aes256Gcm);

impl Cipher {
    /// Build from the ENCRYPTION_KEY env var (base64-encoded 32 bytes).
    /// Generate a key with: openssl rand -base64 32
    pub fn from_base64_key(b64_key: &str) -> Result<Self> {
        let bytes = B64
            .decode(b64_key)
            .map_err(|e| Error::Crypto(format!("bad key encoding: {e}")))?;

        if bytes.len() != 32 {
            return Err(Error::Crypto(format!(
                "key must decode to 32 bytes, got {}",
                bytes.len()
            )));
        }

        let cipher = Aes256Gcm::new_from_slice(&bytes).map_err(|e| Error::Crypto(e.to_string()))?;

        Ok(Self(cipher))
    }

    /// Returns base64(random_nonce || ciphertext).
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = self
            .0
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| Error::Crypto(e.to_string()))?;

        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(B64.encode(combined))
    }

    // Decrypts output from 'encrypt'.
    pub fn decrypt(&self, encoded: &str) -> Result<String> {
        let combined = B64
            .decode(encoded)
            .map_err(|e| Error::Crypto(format!("bas base64: {e}")))?;

        if combined.len() <= 12 {
            return Err(Error::Crypto("ciphertext too short".into()));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .0
            .decrypt(nonce, ciphertext)
            .map_err(|e| Error::Crypto(e.to_string()))?;

        String::from_utf8(plaintext).map_err(|e| Error::Crypto(e.to_string()))
    }
}
