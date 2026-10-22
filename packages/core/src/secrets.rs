use base64::Engine;
use sha2::{Digest, Sha256};

pub const TOKEN_PREFIX: &str = "sst_";

pub fn generate_token() -> String {
  random_secret(TOKEN_PREFIX)
}

pub fn random_secret(prefix: &str) -> String {
  let mut bytes = [0u8; 32];
  rand::fill(&mut bytes);
  format!(
    "{prefix}{}",
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
  )
}

pub fn hash_token(token: &str) -> String {
  Sha256::digest(token.as_bytes())
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect()
}
