use std::time::Duration;

use sdd_vault_core::config::SmtpSettings;
use sdd_vault_core::email::{Mailer, mask_address, render};
use sdd_vault_core::store::Store;
use sdd_vault_core::store::outbox::OutboxMessage;
use tokio::task::JoinHandle;

pub const BATCH: i64 = 20;
pub const INTERVAL: Duration = Duration::from_secs(10);

pub struct Sender {
  store: Store,
  smtp: SmtpSettings,
}

async fn blocking<T: Send + 'static>(
  store: &Store,
  work: impl FnOnce(&Store) -> sdd_vault_core::model::StoreResult<T> + Send + 'static,
) -> Option<T> {
  let store = store.clone();
  match tokio::task::spawn_blocking(move || work(&store)).await {
    Ok(Ok(value)) => Some(value),
    Ok(Err(error)) => {
      tracing::error!(%error, "outbox store call failed");
      None
    }
    Err(error) => {
      tracing::error!(%error, "outbox task failed");
      None
    }
  }
}

impl Sender {
  pub fn new(store: Store, smtp: SmtpSettings) -> Sender {
    Sender { store, smtp }
  }

  async fn failed(&self, message: &OutboxMessage, detail: &str) {
    let id = message.id;
    let recipient = mask_address(&message.recipient);
    match blocking(&self.store, move |store| {
      store.mark_message_attempt_failed(id)
    })
    .await
    {
      Some(true) => {
        tracing::error!(id, %recipient, %detail, "email given up after the retry window")
      }
      _ => {
        tracing::warn!(id, %recipient, %detail, attempts = message.attempts, "email delivery failed; will retry")
      }
    }
  }

  pub async fn run_once(&self) -> usize {
    let Some(messages) = blocking(&self.store, |store| store.claim_due_messages(BATCH)).await
    else {
      return 0;
    };
    if messages.is_empty() {
      return 0;
    }
    let mailer = match Mailer::new(&self.smtp) {
      Ok(mailer) => mailer,
      Err(error) => {
        for message in &messages {
          self.failed(message, &error.to_string()).await;
        }
        return 0;
      }
    };
    let mut sent = 0;
    for message in messages {
      let delivery = match render(&message.kind, &message.language, &message.payload) {
        Ok(rendered) => mailer.send(&message.recipient, &rendered).await,
        Err(error) => Err(error),
      };
      match delivery {
        Ok(()) => {
          let id = message.id;
          if blocking(&self.store, move |store| store.mark_message_sent(id))
            .await
            .is_some()
          {
            sent += 1;
          }
        }
        Err(error) => self.failed(&message, &error.to_string()).await,
      }
    }
    sent
  }

  pub fn spawn(self, every: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
      loop {
        self.run_once().await;
        tokio::time::sleep(every).await;
      }
    })
  }
}
