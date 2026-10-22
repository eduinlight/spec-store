use anyhow::Context;
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use spec_store_core::address::ProjectRef;
use spec_store_core::keycloak::encode_segment;

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
  #[command(subcommand, about = "Projects, project roles and repository URLs")]
  Project(ProjectCommand),
  #[command(subcommand, about = "Audit trail")]
  Audit(AuditCommand),
}

#[derive(Subcommand, Debug)]
pub enum OrgCommand {
  #[command(about = "Create an organization you own")]
  Create { slug: String, name: String },
  #[command(about = "List your organizations")]
  List,
  #[command(about = "List the members of an organization")]
  Members { org: String },
  #[command(about = "Invite an email address to an organization")]
  Invite {
    org: String,
    email: String,
    #[arg(long, default_value = "en")]
    language: String,
  },
  #[command(about = "List the invitations of an organization")]
  Invitations { org: String },
  #[command(about = "Send a pending invitation again")]
  Resend { org: String, id: i64 },
  #[command(name = "revoke-invite", about = "Revoke a pending invitation")]
  RevokeInvite { org: String, id: i64 },
  #[command(subcommand, about = "Change or remove organization members")]
  Member(OrgMemberCommand),
}

#[derive(Subcommand, Debug)]
pub enum OrgMemberCommand {
  #[command(about = "Set a member's organization role: owner or member")]
  Set {
    org: String,
    handle: String,
    role: String,
  },
  #[command(about = "Remove a member, or leave when it is yourself")]
  Remove { org: String, handle: String },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
  #[command(about = "Create a project: spec-store project create <org>/<project> <name>")]
  Create { project: String, name: String },
  #[command(about = "List your projects, optionally in one organization")]
  List { org: Option<String> },
  #[command(about = "Show a project")]
  Show { project: String },
  #[command(about = "List the members of a project")]
  Members { project: String },
  #[command(subcommand, about = "Grant or remove project roles")]
  Member(ProjectMemberCommand),
  #[command(name = "set-repo", about = "Set the repository URL of a project")]
  SetRepo { project: String, url: String },
  #[command(name = "clear-repo", about = "Clear the repository URL of a project")]
  ClearRepo { project: String },
}

#[derive(Subcommand, Debug)]
pub enum ProjectMemberCommand {
  #[command(about = "Grant a project role: owner, editor or reader")]
  Set {
    project: String,
    handle: String,
    role: String,
  },
  #[command(about = "Remove a project role")]
  Remove { project: String, handle: String },
}

#[derive(Subcommand, Debug)]
pub enum AuditCommand {
  #[command(about = "List the audit trail of an organization, newest first")]
  List {
    org: String,
    #[arg(long)]
    project: Option<String>,
    #[arg(long, default_value_t = 50)]
    limit: usize,
  },
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

fn text(value: &Value) -> &str {
  value.as_str().unwrap_or_default()
}

fn items(value: &Value, key: &str) -> Vec<Value> {
  value[key].as_array().cloned().unwrap_or_default()
}

fn segment(value: &str) -> String {
  encode_segment(value)
}

fn project_path(project: &str) -> anyhow::Result<String> {
  let address = ProjectRef::parse(project)?;
  Ok(format!(
    "/api/v1/projects/{}/{}",
    segment(&address.organization),
    segment(&address.project)
  ))
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
  match &cli.command {
    Command::Login => run_login(&cli).await,
    Command::Logout => run_logout(&cli).await,
    Command::Whoami => {
      let me = cli.client()?.get("/api/v1/me").await?;
      let operator = if me["is_operator"] == Value::Bool(true) {
        " (operator)"
      } else {
        ""
      };
      println!("{} <{}>{operator}", text(&me["handle"]), text(&me["email"]));
      Ok(())
    }
    Command::Org(command) => run_org(&mut cli.client()?, command).await,
    Command::Project(command) => run_project(&mut cli.client()?, command).await,
    Command::Audit(AuditCommand::List {
      org,
      project,
      limit,
    }) => {
      let mut path = format!("/api/v1/orgs/{}/audit?limit={limit}", segment(org));
      if let Some(project) = project {
        path.push_str(&format!("&project={}", segment(project)));
      }
      let page = cli.client()?.get(&path).await?;
      for entry in items(&page, "entries") {
        println!(
          "{}\t{}\t{}\t{}\t{}",
          text(&entry["created_at"]),
          text(&entry["actor"]),
          text(&entry["action"]),
          entry["project"]
            .as_str()
            .or(entry["organization"].as_str())
            .unwrap_or("-"),
          text(&entry["target"])
        );
      }
      Ok(())
    }
  }
}

async fn run_org(client: &mut ApiClient, command: &OrgCommand) -> anyhow::Result<()> {
  match command {
    OrgCommand::Create { slug, name } => {
      let created = client
        .post("/api/v1/orgs", &json!({ "slug": slug, "name": name }))
        .await?;
      println!("{}\t{}", text(&created["slug"]), text(&created["name"]));
    }
    OrgCommand::List => {
      let listed = client.get("/api/v1/orgs").await?;
      for org in items(&listed, "organizations") {
        println!(
          "{}\t{}\t{}",
          text(&org["slug"]),
          org["role"].as_str().unwrap_or("operator"),
          text(&org["name"])
        );
      }
    }
    OrgCommand::Members { org } => {
      let listed = client
        .get(&format!("/api/v1/orgs/{}/members", segment(org)))
        .await?;
      for member in items(&listed, "members") {
        println!(
          "{}\t{}\t{}",
          text(&member["handle"]),
          text(&member["role"]),
          text(&member["name"])
        );
      }
    }
    OrgCommand::Invite {
      org,
      email,
      language,
    } => {
      let created = client
        .post(
          &format!("/api/v1/orgs/{}/invitations", segment(org)),
          &json!({ "email": email, "language": language }),
        )
        .await?;
      println!(
        "{}\t{}\texpires {}",
        created["id"],
        text(&created["email"]),
        text(&created["expires_at"])
      );
    }
    OrgCommand::Invitations { org } => {
      let listed = client
        .get(&format!("/api/v1/orgs/{}/invitations", segment(org)))
        .await?;
      for invitation in items(&listed, "invitations") {
        println!(
          "{}\t{}\t{}\t{}\t{}",
          invitation["id"],
          text(&invitation["email"]),
          text(&invitation["state"]),
          text(&invitation["inviter"]),
          text(&invitation["expires_at"])
        );
      }
    }
    OrgCommand::Resend { org, id } => {
      client
        .post(
          &format!("/api/v1/orgs/{}/invitations/{id}/resend", segment(org)),
          &Value::Null,
        )
        .await?;
      println!("Invitation {id} sent again");
    }
    OrgCommand::RevokeInvite { org, id } => {
      client
        .delete(&format!("/api/v1/orgs/{}/invitations/{id}", segment(org)))
        .await?;
      println!("Invitation {id} revoked");
    }
    OrgCommand::Member(OrgMemberCommand::Set { org, handle, role }) => {
      client
        .put(
          &format!("/api/v1/orgs/{}/members/{}", segment(org), segment(handle)),
          &json!({ "role": role }),
        )
        .await?;
      println!("{handle} is now {role} of {org}");
    }
    OrgCommand::Member(OrgMemberCommand::Remove { org, handle }) => {
      client
        .delete(&format!(
          "/api/v1/orgs/{}/members/{}",
          segment(org),
          segment(handle)
        ))
        .await?;
      println!("{handle} removed from {org}");
    }
  }
  Ok(())
}

async fn print_projects(client: &mut ApiClient, org: &str) -> anyhow::Result<()> {
  let listed = client
    .get(&format!("/api/v1/orgs/{}/projects", segment(org)))
    .await?;
  for project in items(&listed, "projects") {
    println!(
      "{}\t{}\t{}",
      text(&project["address"]),
      project["role"].as_str().unwrap_or("operator"),
      text(&project["name"])
    );
  }
  Ok(())
}

async fn run_project(client: &mut ApiClient, command: &ProjectCommand) -> anyhow::Result<()> {
  match command {
    ProjectCommand::Create { project, name } => {
      let address = ProjectRef::parse(project)?;
      let created = client
        .post(
          &format!("/api/v1/orgs/{}/projects", segment(&address.organization)),
          &json!({ "slug": address.project, "name": name }),
        )
        .await?;
      println!("{}\t{}", text(&created["address"]), text(&created["name"]));
    }
    ProjectCommand::List { org: Some(org) } => print_projects(client, org).await?,
    ProjectCommand::List { org: None } => {
      let orgs = client.get("/api/v1/orgs").await?;
      for org in items(&orgs, "organizations") {
        print_projects(client, text(&org["slug"])).await?;
      }
    }
    ProjectCommand::Show { project } => {
      let shown = client.get(&project_path(project)?).await?;
      println!("address\t{}", text(&shown["address"]));
      println!("name\t{}", text(&shown["name"]));
      println!("role\t{}", shown["role"].as_str().unwrap_or("operator"));
      println!(
        "repository\t{}",
        shown["repository_url"].as_str().unwrap_or("-")
      );
      println!("created\t{}", text(&shown["created_at"]));
    }
    ProjectCommand::Members { project } => {
      let listed = client
        .get(&format!("{}/members", project_path(project)?))
        .await?;
      for member in items(&listed, "members") {
        println!("{}\t{}", text(&member["handle"]), text(&member["role"]));
      }
    }
    ProjectCommand::Member(ProjectMemberCommand::Set {
      project,
      handle,
      role,
    }) => {
      client
        .put(
          &format!("{}/members/{}", project_path(project)?, segment(handle)),
          &json!({ "role": role }),
        )
        .await?;
      println!("{handle} is now {role} of {project}");
    }
    ProjectCommand::Member(ProjectMemberCommand::Remove { project, handle }) => {
      client
        .delete(&format!(
          "{}/members/{}",
          project_path(project)?,
          segment(handle)
        ))
        .await?;
      println!("{handle} removed from {project}");
    }
    ProjectCommand::SetRepo { project, url } => {
      client
        .put(
          &format!("{}/repository-url", project_path(project)?),
          &json!({ "url": url }),
        )
        .await?;
      println!("Repository of {project} set to {url}");
    }
    ProjectCommand::ClearRepo { project } => {
      client
        .delete(&format!("{}/repository-url", project_path(project)?))
        .await?;
      println!("Repository of {project} cleared");
    }
  }
  Ok(())
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
  let handle = text(&me["handle"]).to_string();
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
