use std::path::PathBuf;

use anyhow::{Context, anyhow, bail};
use clap::{Args, Parser, Subcommand};

use crate::model::{Caller, Role, StoreError};
use crate::server::{self, ServerConfig};
use crate::store::Store;
use crate::store::transfer::{ImportOptions, ImportReport};
use crate::tree::{self, TargetState};

#[derive(Parser, Debug)]
#[command(
  name = "spec-store",
  version,
  about = "Remote, multi-project store for Spec Driven Development documents"
)]
pub struct Cli {
  #[arg(
    long,
    global = true,
    default_value = "spec-store.db",
    env = "SPEC_STORE_DB"
  )]
  pub db: PathBuf,
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  #[command(about = "Serve the MCP endpoint over HTTP")]
  Serve(ServeArgs),
  #[command(about = "Serve the MCP interface over stdin and stdout acting for one user")]
  Stdio(StdioArgs),
  #[command(subcommand, about = "Manage users")]
  User(UserCmd),
  #[command(subcommand, about = "Manage API tokens")]
  Token(TokenCmd),
  #[command(subcommand, about = "Manage projects")]
  Project(ProjectCmd),
  #[command(subcommand, about = "Manage project memberships")]
  Member(MemberCmd),
  #[command(subcommand, about = "Read the audit trail")]
  Audit(AuditCmd),
  #[command(about = "Import docs/constitution.md and specs/ from a project directory")]
  Import(ImportArgs),
  #[command(about = "Export the current documents of a project to a directory")]
  Export(ExportArgs),
}

#[derive(Args, Debug)]
pub struct ExportArgs {
  pub project: String,
  pub dir: PathBuf,
  #[arg(long = "spec", value_name = "NUMBER")]
  pub spec: Option<i64>,
  #[arg(long)]
  pub force: bool,
  #[arg(long = "as", value_name = "HANDLE")]
  pub as_handle: String,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
  pub project: String,
  pub dir: PathBuf,
  #[arg(long)]
  pub constitution_version: Option<String>,
  #[arg(long)]
  pub dry_run: bool,
  #[arg(long = "as", value_name = "HANDLE")]
  pub as_handle: String,
}

#[derive(Args, Debug)]
pub struct ServeArgs {
  #[arg(long, default_value = "127.0.0.1:7391")]
  pub bind: String,
  #[arg(long = "allowed-host")]
  pub allowed_hosts: Vec<String>,
  #[arg(long)]
  pub secure_cookies: bool,
}

#[derive(Args, Debug)]
pub struct StdioArgs {
  #[arg(long = "as", value_name = "HANDLE")]
  pub as_handle: String,
}

#[derive(Subcommand, Debug)]
pub enum UserCmd {
  #[command(about = "Create a user; the first user of an empty store becomes admin")]
  Add {
    handle: String,
    #[arg(long)]
    admin: bool,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: Option<String>,
  },
}

#[derive(Subcommand, Debug)]
pub enum TokenCmd {
  #[command(about = "Issue a token for a user and print it once")]
  Issue {
    handle: String,
    #[arg(long)]
    label: String,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
  #[command(about = "Revoke a token by id")]
  Revoke {
    id: i64,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
  #[command(about = "List the tokens of a user without their secrets")]
  List {
    handle: String,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCmd {
  #[command(about = "Create a project")]
  Add {
    slug: String,
    name: String,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
  #[command(about = "Set or clear the repository URL used to link commits in the browser view")]
  SetRepo {
    slug: String,
    url: Option<String>,
    #[arg(long)]
    clear: bool,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
}

#[derive(Subcommand, Debug)]
pub enum MemberCmd {
  #[command(about = "Set the role of a user in a project (owner, editor, reader)")]
  Set {
    project: String,
    handle: String,
    role: String,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
  #[command(about = "Remove a user from a project")]
  Remove {
    project: String,
    handle: String,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
}

#[derive(Subcommand, Debug)]
pub enum AuditCmd {
  #[command(about = "List audit entries, newest first")]
  List {
    #[arg(long)]
    project: Option<String>,
    #[arg(long, default_value_t = 50)]
    limit: usize,
    #[arg(long = "as", value_name = "HANDLE")]
    as_handle: String,
  },
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
  let store = Store::open(&cli.db)
    .map_err(|error| anyhow::anyhow!("opening {}: {error}", cli.db.display()))?;
  match cli.command {
    Command::Serve(args) => runtime()?.block_on(server::serve(
      store,
      &args.bind,
      ServerConfig {
        allowed_hosts: args.allowed_hosts,
        secure_cookies: args.secure_cookies,
      },
    )),
    Command::Stdio(args) => {
      let caller = actor(&store, &args.as_handle)?;
      runtime()?.block_on(server::run_stdio(store, caller))
    }
    Command::User(cmd) => run_user(&store, cmd),
    Command::Token(cmd) => run_token(&store, cmd),
    Command::Project(cmd) => run_project(&store, cmd),
    Command::Member(cmd) => run_member(&store, cmd),
    Command::Audit(cmd) => run_audit(&store, cmd),
    Command::Import(args) => run_import(&store, args),
    Command::Export(args) => run_export(&store, args),
  }
}

fn runtime() -> anyhow::Result<tokio::runtime::Runtime> {
  tokio::runtime::Runtime::new().context("starting the async runtime")
}

fn actor(store: &Store, handle: &str) -> anyhow::Result<Caller> {
  store
    .caller_by_handle(handle)
    .map_err(|_| anyhow!("acting user {handle:?} does not exist"))
}

fn require_admin_actor(store: &Store, handle: &str) -> anyhow::Result<Caller> {
  let caller = actor(store, handle)?;
  if !caller.is_admin {
    bail!("acting user {handle:?} is not an admin");
  }
  Ok(caller)
}

fn run_user(store: &Store, cmd: UserCmd) -> anyhow::Result<()> {
  match cmd {
    UserCmd::Add {
      handle,
      admin,
      as_handle,
    } => {
      let acting = as_handle.as_deref().map(|h| actor(store, h)).transpose()?;
      let user = match store.create_user(acting.as_ref(), &handle, admin) {
        Err(StoreError::Forbidden) if acting.is_none() => {
          bail!("an acting admin is required: pass --as <handle>")
        }
        Err(StoreError::Forbidden) => bail!("acting user is not an admin"),
        other => other?,
      };
      let suffix = if user.is_admin { " (admin)" } else { "" };
      println!("created user {}{suffix}", user.handle);
      Ok(())
    }
  }
}

fn run_token(store: &Store, cmd: TokenCmd) -> anyhow::Result<()> {
  match cmd {
    TokenCmd::Issue {
      handle,
      label,
      as_handle,
    } => {
      let admin = require_admin_actor(store, &as_handle)?;
      let issued = store.issue_token(Some(&admin), &handle, &label)?;
      eprintln!(
        "issued token {} for {handle} ({label}); it is shown only once",
        issued.id
      );
      println!("{}", issued.token);
      Ok(())
    }
    TokenCmd::Revoke { id, as_handle } => {
      let admin = require_admin_actor(store, &as_handle)?;
      store.revoke_token(Some(&admin), id)?;
      println!("revoked token {id}");
      Ok(())
    }
    TokenCmd::List { handle, as_handle } => {
      let admin = require_admin_actor(store, &as_handle)?;
      for token in store.list_tokens(Some(&admin), &handle)? {
        let state = match &token.revoked_at {
          Some(at) => format!("revoked {at}"),
          None => "active".to_string(),
        };
        let last_used = token.last_used_at.as_deref().unwrap_or("never");
        println!(
          "{}\t{}\tcreated {}\tlast used {last_used}\t{state}",
          token.id, token.label, token.created_at
        );
      }
      Ok(())
    }
  }
}

fn run_project(store: &Store, cmd: ProjectCmd) -> anyhow::Result<()> {
  match cmd {
    ProjectCmd::Add {
      slug,
      name,
      as_handle,
    } => {
      let admin = require_admin_actor(store, &as_handle)?;
      let project = store.create_project(&admin, &slug, &name)?;
      println!("created project {} ({})", project.slug, project.name);
      Ok(())
    }
    ProjectCmd::SetRepo {
      slug,
      url,
      clear,
      as_handle,
    } => {
      let acting = actor(store, &as_handle)?;
      match (url, clear) {
        (Some(url), false) => {
          store.set_repository_url(&acting, &slug, Some(&url))?;
          println!("set repository of {slug} to {url}");
        }
        (None, true) => {
          store.set_repository_url(&acting, &slug, None)?;
          println!("cleared repository of {slug}");
        }
        _ => bail!("pass a repository URL or --clear"),
      }
      Ok(())
    }
  }
}

fn run_member(store: &Store, cmd: MemberCmd) -> anyhow::Result<()> {
  match cmd {
    MemberCmd::Set {
      project,
      handle,
      role,
      as_handle,
    } => {
      let acting = actor(store, &as_handle)?;
      let role: Role = role.parse()?;
      store.set_member(&acting, &project, &handle, role)?;
      println!("set {handle} as {role} in {project}");
      Ok(())
    }
    MemberCmd::Remove {
      project,
      handle,
      as_handle,
    } => {
      let acting = actor(store, &as_handle)?;
      store.remove_member(&acting, &project, &handle)?;
      println!("removed {handle} from {project}");
      Ok(())
    }
  }
}

fn run_audit(store: &Store, cmd: AuditCmd) -> anyhow::Result<()> {
  match cmd {
    AuditCmd::List {
      project,
      limit,
      as_handle,
    } => {
      require_admin_actor(store, &as_handle)?;
      for entry in store.list_audit(project.as_deref(), limit)? {
        let scope = entry
          .project
          .as_deref()
          .map(|p| format!(" [{p}]"))
          .unwrap_or_default();
        println!(
          "{} {} {} {}{scope}",
          entry.created_at, entry.actor, entry.action, entry.target
        );
      }
      Ok(())
    }
  }
}

fn problem_list(heading: &str, problems: &str) -> anyhow::Error {
  let lines: Vec<String> = problems.lines().map(|p| format!("  - {p}")).collect();
  anyhow!("{heading}\n{}", lines.join("\n"))
}

fn print_import_report(report: &ImportReport, dry_run: bool) {
  for branch in &report.status_fallbacks {
    eprintln!("notice: spec {branch} has no recognised Status line; imported as draft");
  }
  if let Some(version) = &report.constitution_version {
    println!("constitution v{version} from docs/constitution.md");
  }
  for spec in &report.specs {
    let kinds = if spec.kinds.is_empty() {
      "no documents".to_string()
    } else {
      spec
        .kinds
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(", ")
    };
    println!("spec {} {}: {kinds}", spec.branch, spec.status);
  }
  let summary = format!(
    "{} specs and {} documents",
    report.specs.len(),
    report.documents
  );
  if dry_run {
    println!("dry run: would import {summary}");
  } else {
    println!("imported {summary}");
  }
}

fn run_import(store: &Store, args: ImportArgs) -> anyhow::Result<()> {
  let acting = actor(store, &args.as_handle)?;
  let tree = tree::scan(&args.dir).with_context(|| format!("reading {}", args.dir.display()))?;
  for path in &tree.skipped {
    eprintln!("skipped {path}");
  }
  let options = ImportOptions {
    constitution_version: args.constitution_version,
    dry_run: args.dry_run,
  };
  let report = match store.import_tree(&acting, &args.project, &tree, &options) {
    Err(StoreError::Forbidden) => bail!(
      "acting user {:?} is not an owner of project {:?}",
      args.as_handle,
      args.project
    ),
    Err(StoreError::Validation(problems)) => {
      return Err(problem_list("import refused:", &problems));
    }
    other => other?,
  };
  print_import_report(&report, args.dry_run);
  Ok(())
}

fn run_export(store: &Store, args: ExportArgs) -> anyhow::Result<()> {
  let acting = actor(store, &args.as_handle)?;
  let export = match store.export_documents(&acting, &args.project, args.spec) {
    Err(StoreError::Forbidden) => bail!(
      "acting user {:?} is not a member of project {:?}",
      args.as_handle,
      args.project
    ),
    other => other?,
  };
  let plan = tree::plan_export(&args.dir, export.files)
    .with_context(|| format!("reading {}", args.dir.display()))?;
  let different = plan.different();
  if !different.is_empty() && !args.force {
    return Err(problem_list(
      "export refused, files differ (use --force to overwrite):",
      &different.join("\n"),
    ));
  }
  tree::write_export(&plan, args.force)
    .with_context(|| format!("writing {}", args.dir.display()))?;
  for notice in &export.notices {
    eprintln!(
      "notice: spec {} is {} in the store but its spec.md says {}",
      notice.branch, notice.stored, notice.written
    );
  }
  let mut written = 0;
  for (file, state) in &plan.targets {
    if *state == TargetState::Unchanged {
      println!("unchanged {}", file.relative_path);
    } else {
      written += 1;
      println!("written {}", file.relative_path);
    }
  }
  println!(
    "exported {} files: {written} written, {} unchanged",
    plan.targets.len(),
    plan.targets.len() - written
  );
  Ok(())
}
