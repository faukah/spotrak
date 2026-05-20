use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use secrecy::ExposeSecret;
use sha2::{Digest, Sha256};

use crate::{
    config::Config,
    error::{AppError, Result},
};

const PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;

pub fn encrypt_spotify_token(config: &Config, plaintext: &str) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(&encryption_key(config))
        .map_err(|err| AppError::internal(err.to_string()))?;
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|err| AppError::internal(err.to_string()))?;

    let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(payload)))
}

pub fn decrypt_spotify_token(config: &Config, value: &str) -> Result<String> {
    let Some(encoded) = value.strip_prefix(PREFIX) else {
        return Err(AppError::internal("Spotify token is not encrypted"));
    };

    let payload = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|err| AppError::internal(format!("invalid encrypted Spotify token: {err}")))?;
    if payload.len() <= NONCE_LEN {
        return Err(AppError::internal("invalid encrypted Spotify token"));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(&encryption_key(config))
        .map_err(|err| AppError::internal(err.to_string()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|err| AppError::internal(err.to_string()))?;
    String::from_utf8(plaintext).map_err(|err| AppError::internal(err.to_string()))
}

fn encryption_key(config: &Config) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"spotrak.spotify-token-encryption.v1");
    hasher.update(config.spotify_secret.expose_secret().as_bytes());
    hasher.finalize().into()
}
