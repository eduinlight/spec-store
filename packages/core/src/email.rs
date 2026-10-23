use std::time::Duration;

use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde_json::Value;

use crate::config::SmtpSettings;
use crate::store::invitations::INVITATION_EMAIL;

pub const SMTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
  #[error("cannot build the email: {0}")]
  Build(String),
  #[error("cannot deliver the email: {0}")]
  Delivery(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmail {
  pub subject: String,
  pub text: String,
  pub html: String,
}

fn escape(text: &str) -> String {
  text
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#39;")
}

fn field<'a>(payload: &'a Value, name: &str) -> &'a str {
  payload[name].as_str().unwrap_or_default()
}

pub fn invitation_email(language: &str, payload: &Value) -> RenderedEmail {
  let organization = field(payload, "organization_name");
  let inviter = field(payload, "inviter");
  let inviter_name = field(payload, "inviter_name");
  let url = field(payload, "accept_url");
  let expires = field(payload, "expires_at");
  let (subject, intro, action, expiry) = if language == "es" {
    (
      format!("Te invitaron a unirte a {organization} en spec-store"),
      format!("{inviter_name} ({inviter}) te invitó a unirte a {organization} en spec-store."),
      "Aceptar la invitación",
      format!("La invitación vence el {expires}."),
    )
  } else {
    (
      format!("You are invited to join {organization} on spec-store"),
      format!("{inviter_name} ({inviter}) invited you to join {organization} on spec-store."),
      "Accept the invitation",
      format!("The invitation expires on {expires}."),
    )
  };
  let text = format!("{intro}\n\n{action}: {url}\n\n{expiry}\n");
  let html = format!(
    "<!doctype html><html><body style=\"font-family:sans-serif\"><p>{}</p><p><a href=\"{}\">{}</a></p><p>{}</p></body></html>",
    escape(&intro),
    escape(url),
    escape(action),
    escape(&expiry)
  );
  RenderedEmail {
    subject,
    text,
    html,
  }
}

pub fn render(kind: &str, language: &str, payload: &str) -> Result<RenderedEmail, EmailError> {
  let payload: Value =
    serde_json::from_str(payload).map_err(|e| EmailError::Build(e.to_string()))?;
  match kind {
    INVITATION_EMAIL => Ok(invitation_email(language, &payload)),
    other => Err(EmailError::Build(format!("unknown email kind {other:?}"))),
  }
}

pub fn mask_address(address: &str) -> String {
  match address.split_once('@') {
    Some((local, domain)) => {
      let first: String = local.chars().take(1).collect();
      format!("{first}***@{domain}")
    }
    None => "***".into(),
  }
}

pub struct Mailer {
  transport: AsyncSmtpTransport<Tokio1Executor>,
  from: Mailbox,
}

impl Mailer {
  pub fn new(settings: &SmtpSettings) -> Result<Mailer, EmailError> {
    let mut builder = if settings.starttls {
      AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.host)
        .map_err(|e| EmailError::Build(e.to_string()))?
    } else {
      AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
    };
    builder = builder.port(settings.port).timeout(Some(SMTP_TIMEOUT));
    if let (Some(username), Some(password)) = (&settings.username, &settings.password) {
      builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
    }
    let from = format!("{} <{}>", settings.from_name, settings.from)
      .parse()
      .map_err(|e: lettre::address::AddressError| EmailError::Build(e.to_string()))?;
    Ok(Mailer {
      transport: builder.build(),
      from,
    })
  }

  pub async fn send(&self, to: &str, email: &RenderedEmail) -> Result<(), EmailError> {
    let to: Mailbox = to
      .parse()
      .map_err(|e: lettre::address::AddressError| EmailError::Build(e.to_string()))?;
    let message = Message::builder()
      .from(self.from.clone())
      .to(to)
      .subject(email.subject.clone())
      .multipart(MultiPart::alternative_plain_html(
        email.text.clone(),
        email.html.clone(),
      ))
      .map_err(|e| EmailError::Build(e.to_string()))?;
    self
      .transport
      .send(message)
      .await
      .map(|_| ())
      .map_err(|e| EmailError::Delivery(e.to_string()))
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn payload() -> Value {
    json!({
      "organization": "acme",
      "organization_name": "Acme <Labs>",
      "inviter": "alice",
      "inviter_name": "Alice",
      "accept_url": "https://app.example/invitations/inv_x",
      "expires_at": "2026-10-29T10:00:00.000Z",
    })
  }

  #[test]
  fn invitation_is_rendered_in_english_and_spanish() {
    let en = invitation_email("en", &payload());
    assert_eq!(
      en.subject,
      "You are invited to join Acme <Labs> on spec-store"
    );
    assert!(en.text.contains("Alice (alice) invited you"));
    assert!(en.text.contains("https://app.example/invitations/inv_x"));
    assert!(en.html.contains("Acme &lt;Labs&gt;"));
    let es = invitation_email("es", &payload());
    assert!(es.subject.starts_with("Te invitaron a unirte a Acme"));
    assert!(es.text.contains("Aceptar la invitación"));
  }

  #[test]
  fn unknown_kinds_are_refused_and_addresses_masked() {
    assert!(render("newsletter", "en", "{}").is_err());
    assert_eq!(mask_address("alice@example.com"), "a***@example.com");
    assert_eq!(mask_address("broken"), "***");
  }
}
