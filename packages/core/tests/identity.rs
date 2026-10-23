use spec_store_core::identity::{IdentityConfig, IdentityError, Verifier};
use spec_store_testkit::{FakeKeycloak, TestKey, TokenSpec};

const AUDIENCE: &str = "https://mcp.example.test/mcp";

fn verifier(keycloak: &FakeKeycloak) -> Verifier {
  Verifier::new(IdentityConfig {
    issuer: keycloak.issuer(),
    jwks_url: keycloak.jwks_url(),
  })
}

#[tokio::test]
async fn valid_token_yields_identity_and_roles() {
  let keycloak = FakeKeycloak::start().await;
  let verifier = verifier(&keycloak);
  let token = keycloak.mint(&TokenSpec::new("alice", AUDIENCE).operator());
  let claims = verifier.verify(&token, AUDIENCE).await.expect("valid");
  assert_eq!(claims.sub, "subject-alice");
  assert_eq!(claims.preferred_username, "alice");
  assert_eq!(claims.email, "alice@example.com");
  assert!(claims.is_operator());
  let identity = claims.identity();
  assert_eq!(identity.handle, "alice");
  assert_eq!(identity.subject, "subject-alice");
}

#[tokio::test]
async fn token_past_expiry_is_rejected_without_leeway() {
  let keycloak = FakeKeycloak::start().await;
  let verifier = verifier(&keycloak);
  let token = keycloak.mint(&TokenSpec::new("alice", AUDIENCE).ttl(-1));
  assert!(matches!(
    verifier.verify(&token, AUDIENCE).await,
    Err(IdentityError::Expired)
  ));
}

#[tokio::test]
async fn wrong_audience_is_rejected() {
  let keycloak = FakeKeycloak::start().await;
  let verifier = verifier(&keycloak);
  let token = keycloak.mint(&TokenSpec::new("alice", "spec-store-api"));
  assert!(matches!(
    verifier.verify(&token, AUDIENCE).await,
    Err(IdentityError::Invalid(_))
  ));
}

#[tokio::test]
async fn wrong_issuer_is_rejected() {
  let keycloak = FakeKeycloak::start().await;
  let verifier = verifier(&keycloak);
  let token =
    keycloak.mint(&TokenSpec::new("alice", AUDIENCE).issuer("https://elsewhere/realms/x"));
  assert!(matches!(
    verifier.verify(&token, AUDIENCE).await,
    Err(IdentityError::Invalid(_))
  ));
}

#[tokio::test]
async fn tampered_token_is_rejected() {
  let keycloak = FakeKeycloak::start().await;
  let verifier = verifier(&keycloak);
  let token = keycloak.mint(&TokenSpec::new("alice", AUDIENCE));
  let mut parts: Vec<String> = token.split('.').map(str::to_string).collect();
  let other = keycloak.mint(&TokenSpec::new("mallory", AUDIENCE));
  parts[1] = other.split('.').nth(1).unwrap().to_string();
  let forged = parts.join(".");
  assert!(matches!(
    verifier.verify(&forged, AUDIENCE).await,
    Err(IdentityError::Invalid(_))
  ));
}

#[tokio::test]
async fn unknown_key_triggers_one_key_set_refresh() {
  let keycloak = FakeKeycloak::start().await;
  let verifier = verifier(&keycloak);
  verifier
    .verify(&keycloak.mint(&TokenSpec::new("alice", AUDIENCE)), AUDIENCE)
    .await
    .expect("key a");
  assert_eq!(keycloak.jwks_fetches(), 1);
  keycloak.publish(TestKey::B);
  let rotated = keycloak.mint(&TokenSpec::new("alice", AUDIENCE).key(TestKey::B));
  verifier
    .verify(&rotated, AUDIENCE)
    .await
    .expect("key b after refresh");
  assert_eq!(keycloak.jwks_fetches(), 2);
  verifier
    .verify(&rotated, AUDIENCE)
    .await
    .expect("key b cached");
  assert_eq!(keycloak.jwks_fetches(), 2);
}

#[tokio::test]
async fn unreachable_key_set_is_unavailable() {
  let verifier = Verifier::new(IdentityConfig {
    issuer: "http://127.0.0.1:1/realms/spec-store".into(),
    jwks_url: "http://127.0.0.1:1/realms/spec-store/protocol/openid-connect/certs".into(),
  });
  let keycloak = FakeKeycloak::start().await;
  let token = keycloak.mint(&TokenSpec::new("alice", AUDIENCE));
  assert!(matches!(
    verifier.verify(&token, AUDIENCE).await,
    Err(IdentityError::Unavailable(_))
  ));
  assert!(!verifier.reachable().await);
}
