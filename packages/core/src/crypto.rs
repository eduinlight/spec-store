use aws_lc_rs::aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub const KEY_BYTES: usize = 32;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CryptoError {
  #[error("the sealing key must be {KEY_BYTES} bytes encoded in base64")]
  InvalidKey,
  #[error("the sealed value cannot be opened")]
  Unsealable,
}

pub struct Sealer {
  key: LessSafeKey,
  random: SystemRandom,
}

impl Sealer {
  pub fn from_base64(encoded: &str) -> Result<Sealer, CryptoError> {
    let bytes = STANDARD
      .decode(encoded.trim())
      .map_err(|_| CryptoError::InvalidKey)?;
    if bytes.len() != KEY_BYTES {
      return Err(CryptoError::InvalidKey);
    }
    let key = UnboundKey::new(&AES_256_GCM, &bytes).map_err(|_| CryptoError::InvalidKey)?;
    Ok(Sealer {
      key: LessSafeKey::new(key),
      random: SystemRandom::new(),
    })
  }

  pub fn generate_key() -> String {
    let mut bytes = [0u8; KEY_BYTES];
    SystemRandom::new().fill(&mut bytes).expect("system random");
    STANDARD.encode(bytes)
  }

  pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
    let mut nonce = [0u8; NONCE_LEN];
    self.random.fill(&mut nonce).expect("system random");
    let mut sealed = plaintext.to_vec();
    self
      .key
      .seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::empty(),
        &mut sealed,
      )
      .expect("seal");
    let mut output = nonce.to_vec();
    output.extend_from_slice(&sealed);
    output
  }

  pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if sealed.len() <= NONCE_LEN {
      return Err(CryptoError::Unsealable);
    }
    let (nonce, ciphertext) = sealed.split_at(NONCE_LEN);
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| CryptoError::Unsealable)?;
    let mut buffer = ciphertext.to_vec();
    let plaintext = self
      .key
      .open_in_place(nonce, Aad::empty(), &mut buffer)
      .map_err(|_| CryptoError::Unsealable)?;
    Ok(plaintext.to_vec())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sealed_values_open_with_the_same_key_only() {
    let sealer = Sealer::from_base64(&Sealer::generate_key()).unwrap();
    let sealed = sealer.seal(b"refresh-token-value");
    assert_ne!(sealed, b"refresh-token-value");
    assert_eq!(sealer.open(&sealed).unwrap(), b"refresh-token-value");
    assert_ne!(
      sealer.seal(b"same"),
      sealer.seal(b"same"),
      "every seal uses a fresh nonce"
    );
    let other = Sealer::from_base64(&Sealer::generate_key()).unwrap();
    assert!(other.open(&sealed).is_err());
  }

  #[test]
  fn tampered_or_short_ciphertexts_are_refused() {
    let sealer = Sealer::from_base64(&Sealer::generate_key()).unwrap();
    let mut sealed = sealer.seal(b"value");
    let last = sealed.len() - 1;
    sealed[last] ^= 1;
    assert!(sealer.open(&sealed).is_err());
    assert!(sealer.open(b"short").is_err());
    assert!(Sealer::from_base64("not-a-key").is_err());
  }
}
