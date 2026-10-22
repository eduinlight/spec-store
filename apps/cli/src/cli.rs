use anyhow::Context;
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::client::ApiClient;
use crate::credentials::{CredentialStore, Credentials};
use crate::login::{self, Endpoints, now};

pub const DEFAULT_API: &str = "https://api-spec-store.eduindev.com";
pub const DEFAULT_ISSUER: &str = "https://auth-spec-store.eduindev.com/realms/spec-store";

#[derive(Parser, Debug)]
#[command(
  name = "spec-store",
  version,
  about = "Command-line tool for the hosted spec-store"
)]
pub struct Cli {
  #[arg(long, global = true, env = "SPEC_STORE_API", default_value = DEFAULT_API)]
  pub api: String,
  #[arg(long, global = true, env = "SPEC_STORE_ISSUER", default_value = DEFAULT_ISSUER)]
  pub issuer: String,
  #[arg(
    long,
    global = true,
    env = "SPEC_STORE_CLIENT_ID",
    default_value = "spec-store-cli"
  )]
  pub client_id: String,
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  #[command(about = "Sign in through the browser with a device code")]
  Login,
  #[command(about = "Sign out and remove the stored credentials")]
  Logout,
  #[command(about = "Show who is signed in")]
  Whoami,
  #[command(subcommand, about = "Organizations, members and invitations")]
  Org(OrgCommand),
}

#[derive(Subcommand, Debug)]
pub enum OrgCommand {
  #[command(about = "List your organizations")]
  List,
}

impl Cli {
  fn endpoints(&self) -> Endpoints {
    Endpoints {
      issuer: self.issuer.clone(),
      client_id: self.client_id.clone(),
    }
  }

  fn client(&self) -> anyhow::Result<ApiClient> {
    Ok(ApiClient::connect(
      &self.api,
      self.endpoints(),
      CredentialStore::from_env(),
    )?)
  }
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
  match &cli.command {
    Command::Login => run_login(&cli).await,
    Command::Logout => run_logout(&cli).await,
    Command::Whoami => {
      let me = cli.client()?.get("/api/v1/me").await?;
      println!(
        "{} <{}>{}",
        me["handle"].as_str().unwrap_or_default(),
        me["email"].as_str().unwrap_or_default(),
        if me["is_operator"] == Value::Bool(true) {
          " (operator)"
        } else {
          ""
        }
      );
      Ok(())
    }
    Command::Org(OrgCommand::List) => {
      let listed = cli.client()?.get("/api/v1/orgs").await?;
      for org in listed["organizations"]
        .as_array()
        .cloned()
        .unwrap_or_default()
      {
        println!(
          "{}\t{}\t{}",
          org["slug"].as_str().unwrap_or_default(),
          org["role"].as_str().unwrap_or("operator"),
          org["name"].as_str().unwrap_or_default()
        );
      }
      Ok(())
    }
  }
}

async fn run_login(cli: &Cli) -> anyhow::Result<()> {
  let http = ApiClient::http();
  let endpoints = cli.endpoints();
  let device = login::start(&http, &endpoints).await?;
  println!(
    "Open {} and enter the code {}",
    device.verification_uri, device.user_code
  );
  if let Some(complete) = &device.verification_uri_complete {
    println!("Or open {complete}");
  }
  println!("Waiting for approval…");
  let tokens = login::poll(&http, &endpoints, &device).await?;
  let refresh_token = tokens
    .refresh_token
    .clone()
    .context("the identity provider returned no refresh token")?;
  let store = CredentialStore::from_env();
  let mut credentials = Credentials {
    issuer: cli.issuer.clone(),
    api: cli.api.clone(),
    access_token: tokens.access_token,
    refresh_token,
    expires_at: now() + tokens.expires_in,
    handle: None,
  };
  store.save(&credentials)?;
  let me = cli.client()?.get("/api/v1/me").await?;
  let handle = me["handle"].as_str().unwrap_or_default().to_string();
  credentials.handle = Some(handle.clone());
  store.save(&credentials)?;
  println!("Signed in as {handle}");
  Ok(())
}

async fn run_logout(cli: &Cli) -> anyhow::Result<()> {
  let store = CredentialStore::from_env();
  match store.load(&cli.issuer)? {
    Some(credentials) => {
      let ended = login::logout(
        &ApiClient::http(),
        &cli.endpoints(),
        &credentials.refresh_token,
      )
      .await;
      store.remove(&cli.issuer)?;
      ended?;
      println!("Signed out");
    }
    None => println!("Not signed in"),
  }
  Ok(())
}
