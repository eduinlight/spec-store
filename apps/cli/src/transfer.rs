use std::path::Path;

use anyhow::{Context, anyhow, bail};
use serde_json::json;
use spec_store_core::address::ProjectRef;
use spec_store_core::keycloak::encode_segment;
use spec_store_core::store::transfer::{ImportReport, ProjectExport};
use spec_store_core::tree::{self, TargetState};

use crate::client::{ApiClient, ClientError};

fn problem_list(heading: &str, problems: &str) -> anyhow::Error {
  let lines: Vec<String> = problems.lines().map(|p| format!("  - {p}")).collect();
  anyhow!("{heading}\n{}", lines.join("\n"))
}

fn transfer_path(project: &str, action: &str) -> anyhow::Result<String> {
  let address = ProjectRef::parse(project)?;
  Ok(format!(
    "/api/v1/projects/{}/{}/{action}",
    encode_segment(&address.organization),
    encode_segment(&address.project)
  ))
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

pub async fn import(
  client: &mut ApiClient,
  project: &str,
  dir: &Path,
  constitution_version: Option<String>,
  dry_run: bool,
) -> anyhow::Result<()> {
  let path = transfer_path(project, "import")?;
  let tree = tree::scan(dir).with_context(|| format!("reading {}", dir.display()))?;
  for skipped in &tree.skipped {
    eprintln!("skipped {skipped}");
  }
  let body =
    json!({ "tree": tree, "constitution_version": constitution_version, "dry_run": dry_run });
  let response = match client.post(&path, &body).await {
    Err(ClientError::Api { status: 403, .. }) => {
      bail!("you are not an owner of project {project:?}")
    }
    Err(ClientError::Api {
      status: 422,
      message,
      ..
    }) => return Err(problem_list("import refused:", &message)),
    other => other?,
  };
  let report: ImportReport =
    serde_json::from_value(response).context("reading the import report")?;
  print_import_report(&report, dry_run);
  Ok(())
}

pub async fn export(
  client: &mut ApiClient,
  project: &str,
  dir: &Path,
  spec: Option<i64>,
  force: bool,
) -> anyhow::Result<()> {
  let mut path = transfer_path(project, "export")?;
  if let Some(spec) = spec {
    path.push_str(&format!("?spec={spec}"));
  }
  let export: ProjectExport =
    serde_json::from_value(client.get(&path).await?).context("reading the export")?;
  let plan =
    tree::plan_export(dir, export.files).with_context(|| format!("reading {}", dir.display()))?;
  let different = plan.different();
  if !different.is_empty() && !force {
    return Err(problem_list(
      "export refused, files differ (use --force to overwrite):",
      &different.join("\n"),
    ));
  }
  tree::write_export(&plan, force).with_context(|| format!("writing {}", dir.display()))?;
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
