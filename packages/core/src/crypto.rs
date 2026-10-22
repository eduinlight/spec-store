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
