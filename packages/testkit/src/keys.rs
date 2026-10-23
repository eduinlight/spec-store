use std::sync::OnceLock;

use aws_lc_rs::encoding::AsDer;
use aws_lc_rs::rsa::{KeyPair, KeySize};
use aws_lc_rs::signature::KeyPair as _;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};

pub struct GeneratedKey {
  pub pem: String,
  pub modulus: String,
  pub exponent: String,
}

fn generate() -> GeneratedKey {
  let pair = KeyPair::generate(KeySize::Rsa2048).expect("generate test signing key");
  let der = pair.as_der().expect("encode test signing key");
  let encoded = STANDARD.encode(der.as_ref());
  let mut pem = String::from("-----BEGIN PRIVATE KEY-----\n");
  for line in encoded.as_bytes().chunks(64) {
    pem.push_str(std::str::from_utf8(line).expect("base64 is ascii"));
    pem.push('\n');
  }
  pem.push_str("-----END PRIVATE KEY-----\n");
  let public = pair.public_key();
  GeneratedKey {
    pem,
    modulus: URL_SAFE_NO_PAD.encode(public.modulus().big_endian_without_leading_zero()),
    exponent: URL_SAFE_NO_PAD.encode(public.exponent().big_endian_without_leading_zero()),
  }
}

pub fn key_a() -> &'static GeneratedKey {
  static KEY: OnceLock<GeneratedKey> = OnceLock::new();
  KEY.get_or_init(generate)
}

pub fn key_b() -> &'static GeneratedKey {
  static KEY: OnceLock<GeneratedKey> = OnceLock::new();
  KEY.get_or_init(generate)
}
