use std::sync::Arc;

use axum::http::request::Parts;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ErrorCode, Implementation, ServerCapabilities, ServerConfig};
use rmcp::service::RequestContext;
use rmcp::{
  ErrorData as McpError, RoleServer, ServerHandler, schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use sdd_vault_core::clients::{ClientNameResolver, MCP_CLIENT_FALLBACK, resolved_name};
use sdd_vault_core::markdown::tasks::{Malformed, Task};
use sdd_vault_core::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult};
use sdd_vault_core::store::Store;
use sdd_vault_core::store::documents::RevisionMeta;
use sdd_vault_core::store::specs::SpecSummary;
use sdd_vault_core::store::tasks::{ActiveStatus, SpecMarker};
use sdd_vault_core::templates;

use crate::auth::ClientHint;

pub const DEFAULT_CLIENT_NAME: &str = "rmcp";

#[derive(Clone)]
pub struct SddVaultMcp {
  store: Store,
  client_names: Option<Arc<dyn ClientNameResolver>>,
  tool_router: ToolRouter<Self>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ProjectOutput {
  pub slug: String,
  pub name: String,
  pub role: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ProjectsOutput {
  pub projects: Vec<ProjectOutput>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ProjectParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct DocumentParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number, for example 1 for spec 001")]
  pub number: i64,
  #[schemars(description = "Document kind: spec, plan, tasks, research or validation")]
  pub kind: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct TemplateParams {
  #[schemars(description = "Template kind: constitution, spec, plan, tasks or validation")]
  pub kind: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SpecOutput {
  pub number: i64,
  pub slug: String,
  pub status: String,
  pub branch: String,
  pub kinds: Vec<String>,
}

impl From<SpecSummary> for SpecOutput {
  fn from(s: SpecSummary) -> Self {
    SpecOutput {
      number: s.number,
      slug: s.slug,
      status: s.status.as_str().to_string(),
      branch: s.branch,
      kinds: s.kinds.iter().map(|k| k.as_str().to_string()).collect(),
    }
  }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SpecsOutput {
  pub specs: Vec<SpecOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ActiveSpecOutput {
  pub spec: Option<SpecOutput>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub message: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct RevisionOutput {
  pub id: i64,
  pub author: String,
  pub created_at: String,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
}

impl From<RevisionMeta> for RevisionOutput {
  fn from(r: RevisionMeta) -> Self {
    RevisionOutput {
      id: r.id,
      author: r.author,
      created_at: r.created_at,
      message: r.message,
      version: r.version,
    }
  }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct DocumentOutput {
  pub body: String,
  pub revision: RevisionOutput,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ConstitutionOutput {
  pub body: String,
  pub version: String,
  pub revision: RevisionOutput,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct TemplateOutput {
  pub kind: String,
  pub body: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CreateSpecParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec slug: one to four lowercase words joined by hyphens")]
  pub slug: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct PutDocumentParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number")]
  pub number: i64,
  #[schemars(description = "Document kind: spec, plan, tasks, research or validation")]
  pub kind: String,
  #[schemars(description = "Full markdown body of the document")]
  pub body: String,
  #[schemars(description = "Short message describing the change")]
  pub message: String,
  #[schemars(
    description = "Revision id the write is based on; mandatory when the document already exists"
  )]
  pub expected_revision: Option<i64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct PutConstitutionParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Full markdown body of the constitution")]
  pub body: String,
  #[schemars(description = "New version, which must differ from the current one")]
  pub version: String,
  #[schemars(description = "Short message describing the change")]
  pub message: String,
  #[schemars(
    description = "Revision id the write is based on; mandatory when a constitution already exists"
  )]
  pub expected_revision: Option<i64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SetStatusParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number")]
  pub number: i64,
  #[schemars(description = "Target status: draft, clarified, approved or archived")]
  pub status: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct MarkTaskParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number")]
  pub number: i64,
  #[schemars(description = "Task identifier such as T05")]
  pub task_id: String,
  #[schemars(description = "Commit identifier of 7 to 40 lowercase hexadecimal characters")]
  pub commit: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct CreatedSpecOutput {
  pub number: i64,
  pub slug: String,
  pub branch: String,
  pub status: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct RevisionIdOutput {
  pub revision_id: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct StatusOutput {
  pub number: i64,
  pub status: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct MarkedTaskOutput {
  pub revision_id: i64,
  pub line: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SpecParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number")]
  pub number: i64,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ListTasksParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number")]
  pub number: i64,
  #[schemars(description = "Return only tasks that are not done")]
  #[serde(default)]
  pub pending_only: bool,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct TaskOutput {
  pub id: String,
  pub phase: String,
  pub parallel: bool,
  pub frs: Vec<String>,
  pub description: String,
  pub done_when: String,
  pub done: bool,
  pub commit: Option<String>,
  pub line: usize,
}

impl From<Task> for TaskOutput {
  fn from(t: Task) -> Self {
    TaskOutput {
      id: t.id,
      phase: t.phase,
      parallel: t.parallel,
      frs: t.frs,
      description: t.description,
      done_when: t.done_when,
      done: t.done,
      commit: t.commit,
      line: t.line,
    }
  }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct MalformedOutput {
  pub line: usize,
  pub text: String,
}

impl From<Malformed> for MalformedOutput {
  fn from(m: Malformed) -> Self {
    MalformedOutput {
      line: m.line,
      text: m.text,
    }
  }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct TasksOutput {
  pub tasks: Vec<TaskOutput>,
  pub malformed: Vec<MalformedOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct MarkerOutput {
  pub document_kind: String,
  pub kind: String,
  pub heading: String,
  pub text: String,
  pub line: usize,
}

impl From<SpecMarker> for MarkerOutput {
  fn from(m: SpecMarker) -> Self {
    MarkerOutput {
      document_kind: m.document_kind.as_str().to_string(),
      kind: m.kind.as_str().to_string(),
      heading: m.heading,
      text: m.text,
      line: m.line,
    }
  }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct MarkersOutput {
  pub markers: Vec<MarkerOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ActiveStatusOutput {
  pub number: i64,
  pub slug: String,
  pub status: String,
  pub branch: String,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub current_phase: Option<String>,
  pub next_task: Option<TaskOutput>,
  pub markers: usize,
}

impl From<ActiveStatus> for ActiveStatusOutput {
  fn from(a: ActiveStatus) -> Self {
    ActiveStatusOutput {
      number: a.number,
      slug: a.slug,
      status: a.status.as_str().to_string(),
      branch: a.branch,
      tasks_done: a.tasks_done,
      tasks_total: a.tasks_total,
      current_phase: a.current_phase,
      next_task: a.next_task.map(TaskOutput::from),
      markers: a.markers,
    }
  }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ProjectStatusOutput {
  pub constitution_version: Option<String>,
  pub specs: Vec<SpecOutput>,
  pub active: Option<ActiveStatusOutput>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ListRevisionsParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Spec number; omit it for the constitution")]
  pub number: Option<i64>,
  #[schemars(
    description = "Document kind: constitution, spec, plan, tasks, research or validation"
  )]
  pub kind: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct GetRevisionParams {
  #[schemars(
    description = "Project address: <organization>/<project>, for example acme/sdd-vault"
  )]
  pub project: String,
  #[schemars(description = "Revision id as returned by the write tools or list_revisions")]
  pub revision_id: i64,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct RevisionsOutput {
  pub revisions: Vec<RevisionOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct RevisionBodyOutput {
  pub body: String,
  pub kind: String,
  pub number: Option<i64>,
  pub revision: RevisionOutput,
}

fn parse_kind(kind: &str) -> Result<DocumentKind, McpError> {
  kind.parse().map_err(to_mcp_error)
}

pub fn to_mcp_error(error: StoreError) -> McpError {
  match error {
    StoreError::Unauthenticated => McpError::new(
      ErrorCode(-32001),
      "authentication required",
      Some(json!({"kind": "unauthenticated"})),
    ),
    StoreError::Forbidden => McpError::new(
      ErrorCode(-32003),
      "not allowed",
      Some(json!({"kind": "forbidden"})),
    ),
    StoreError::NotFound(what) => McpError::resource_not_found(
      format!("{what} not found"),
      Some(json!({"kind": "not_found", "what": what})),
    ),
    StoreError::Conflict { current_revision } => McpError::invalid_params(
      format!("revision conflict: current revision is {current_revision}"),
      Some(json!({"kind": "conflict", "current_revision": current_revision})),
    ),
    StoreError::Validation(message) => {
      McpError::invalid_params(message, Some(json!({"kind": "validation"})))
    }
    StoreError::Malformed(message) => {
      McpError::invalid_params(message, Some(json!({"kind": "malformed"})))
    }
    StoreError::Unavailable(what) => McpError::internal_error(
      format!("{what} is unavailable"),
      Some(json!({"kind": "unavailable"})),
    ),
    StoreError::Database(_) | StoreError::Pool(_) | StoreError::Migration { .. } => {
      tracing::error!(error = %error, "database error");
      McpError::internal_error("database error", Some(json!({"kind": "internal"})))
    }
  }
}

#[tool_router]
impl SddVaultMcp {
  pub fn new(store: Store, client_names: Option<Arc<dyn ClientNameResolver>>) -> Self {
    Self {
      store,
      client_names,
      tool_router: Self::tool_router(),
    }
  }

  async fn run<T, F>(&self, work: F) -> Result<T, McpError>
  where
    T: Send + 'static,
    F: FnOnce(&Store) -> StoreResult<T> + Send + 'static,
  {
    let store = self.store.clone();
    tokio::task::spawn_blocking(move || work(&store))
      .await
      .map_err(|e| McpError::internal_error(e.to_string(), Some(json!({"kind": "internal"}))))?
      .map_err(to_mcp_error)
  }

  async fn caller(&self, ctx: &RequestContext<RoleServer>) -> Result<Caller, McpError> {
    let parts = ctx
      .extensions
      .get::<Parts>()
      .ok_or_else(|| to_mcp_error(StoreError::Unauthenticated))?;
    let mut caller = parts
      .extensions
      .get::<Caller>()
      .cloned()
      .ok_or_else(|| to_mcp_error(StoreError::Unauthenticated))?;
    let named = ctx
      .client_info()
      .map(|info| info.name.trim().to_string())
      .filter(|name| !name.is_empty() && name != DEFAULT_CLIENT_NAME);
    caller.via.client = match (named, parts.extensions.get::<ClientHint>()) {
      (Some(name), _) => name,
      (None, Some(hint)) => {
        resolved_name(
          self.client_names.as_deref(),
          &hint.subject,
          hint.client_id.as_deref(),
          MCP_CLIENT_FALLBACK,
        )
        .await
      }
      (None, None) => MCP_CLIENT_FALLBACK.to_string(),
    };
    Ok(caller)
  }

  #[tool(
    description = "List the projects the caller can open, addressed as <organization>/<project>, with their role; operators see every project"
  )]
  async fn list_projects(
    &self,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<ProjectsOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let projects = self.run(move |store| store.list_projects(&caller)).await?;
    Ok(Json(ProjectsOutput {
      projects: projects
        .into_iter()
        .map(|p| ProjectOutput {
          slug: p.slug,
          name: p.name,
          role: p.role,
        })
        .collect(),
    }))
  }

  #[tool(
    description = "List the specs of a project with number, slug, status, branch and the document kinds present"
  )]
  async fn list_specs(
    &self,
    Parameters(p): Parameters<ProjectParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<SpecsOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let specs = self
      .run(move |store| store.list_specs(&caller, &p.project))
      .await?;
    Ok(Json(SpecsOutput {
      specs: specs.into_iter().map(SpecOutput::from).collect(),
    }))
  }

  #[tool(
    description = "Return the active spec of a project: the non-archived spec with the highest number"
  )]
  async fn get_active_spec(
    &self,
    Parameters(p): Parameters<ProjectParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<ActiveSpecOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let active = self
      .run(move |store| store.active_spec(&caller, &p.project))
      .await?;
    let message = active
      .is_none()
      .then(|| "no active spec: the project has no non-archived specs".to_string());
    Ok(Json(ActiveSpecOutput {
      spec: active.map(SpecOutput::from),
      message,
    }))
  }

  #[tool(
    description = "Read the current body of a spec document (spec, plan, tasks, research or validation) with its revision"
  )]
  async fn get_document(
    &self,
    Parameters(p): Parameters<DocumentParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<DocumentOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let kind = parse_kind(&p.kind)?;
    let doc = self
      .run(move |store| store.get_document(&caller, &p.project, p.number, kind))
      .await?;
    Ok(Json(DocumentOutput {
      body: doc.body,
      revision: doc.revision.into(),
    }))
  }

  #[tool(description = "Read the current constitution of a project with its version")]
  async fn get_constitution(
    &self,
    Parameters(p): Parameters<ProjectParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<ConstitutionOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let c = self
      .run(move |store| store.get_constitution(&caller, &p.project))
      .await?;
    Ok(Json(ConstitutionOutput {
      body: c.body,
      version: c.version,
      revision: c.revision.into(),
    }))
  }

  #[tool(
    description = "Return the global template for a document kind (constitution, spec, plan, tasks, validation)"
  )]
  async fn get_template(
    &self,
    Parameters(p): Parameters<TemplateParams>,
  ) -> Result<Json<TemplateOutput>, McpError> {
    let (kind, body) = templates::get_by_name(&p.kind).map_err(to_mcp_error)?;
    Ok(Json(TemplateOutput {
      kind: kind.as_str().to_string(),
      body: body.to_string(),
    }))
  }

  #[tool(
    description = "Create a spec in a project: assigns the next number, status draft and the branch name NNN-slug"
  )]
  async fn create_spec(
    &self,
    Parameters(p): Parameters<CreateSpecParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<CreatedSpecOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let created = self
      .run(move |store| store.create_spec(&caller, &p.project, &p.slug))
      .await?;
    Ok(Json(CreatedSpecOutput {
      number: created.number,
      slug: created.slug,
      branch: created.branch,
      status: created.status.as_str().to_string(),
    }))
  }

  #[tool(
    description = "Write the full body of a spec document as a new revision; pass expected_revision when the document exists"
  )]
  async fn put_document(
    &self,
    Parameters(p): Parameters<PutDocumentParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<RevisionIdOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let kind = parse_kind(&p.kind)?;
    let revision_id = self
      .run(move |store| {
        store.put_document(
          &caller,
          &p.project,
          p.number,
          kind,
          &p.body,
          &p.message,
          p.expected_revision,
        )
      })
      .await?;
    Ok(Json(RevisionIdOutput { revision_id }))
  }

  #[tool(
    description = "Write the constitution of a project as a new revision with a new version; owners only"
  )]
  async fn put_constitution(
    &self,
    Parameters(p): Parameters<PutConstitutionParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<RevisionIdOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let revision_id = self
      .run(move |store| {
        store.put_constitution(
          &caller,
          &p.project,
          &p.body,
          &p.version,
          &p.message,
          p.expected_revision,
        )
      })
      .await?;
    Ok(Json(RevisionIdOutput { revision_id }))
  }

  #[tool(
    description = "Change the status of a spec: draft→clarified, clarified→approved, back to draft, or archived (owners only)"
  )]
  async fn set_spec_status(
    &self,
    Parameters(p): Parameters<SetStatusParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<StatusOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let status: SpecStatus = p.status.parse().map_err(to_mcp_error)?;
    let number = p.number;
    let status = self
      .run(move |store| store.set_spec_status(&caller, &p.project, p.number, status))
      .await?;
    Ok(Json(StatusOutput {
      number,
      status: status.as_str().to_string(),
    }))
  }

  #[tool(
    description = "Mark a task done in the tasks document, recording the commit on its line as a new revision"
  )]
  async fn mark_task_done(
    &self,
    Parameters(p): Parameters<MarkTaskParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<MarkedTaskOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let marked = self
      .run(move |store| store.mark_task_done(&caller, &p.project, p.number, &p.task_id, &p.commit))
      .await?;
    Ok(Json(MarkedTaskOutput {
      revision_id: marked.revision_id,
      line: marked.line,
    }))
  }

  #[tool(
    description = "List the tasks of a spec parsed from its tasks document, with malformed lines reported separately"
  )]
  async fn list_tasks(
    &self,
    Parameters(p): Parameters<ListTasksParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<TasksOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let parsed = self
      .run(move |store| store.list_tasks(&caller, &p.project, p.number, p.pending_only))
      .await?;
    Ok(Json(TasksOutput {
      tasks: parsed.tasks.into_iter().map(TaskOutput::from).collect(),
      malformed: parsed
        .malformed
        .into_iter()
        .map(MalformedOutput::from)
        .collect(),
    }))
  }

  #[tool(
    description = "List the NEEDS CLARIFICATION and NEEDS RESEARCH markers across the documents of a spec"
  )]
  async fn list_markers(
    &self,
    Parameters(p): Parameters<SpecParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<MarkersOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let markers = self
      .run(move |store| store.list_markers(&caller, &p.project, p.number))
      .await?;
    Ok(Json(MarkersOutput {
      markers: markers.into_iter().map(MarkerOutput::from).collect(),
    }))
  }

  #[tool(
    description = "Status of a project: constitution version, specs, and for the active spec its task progress, next task and markers"
  )]
  async fn project_status(
    &self,
    Parameters(p): Parameters<ProjectParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<ProjectStatusOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let status = self
      .run(move |store| store.project_status(&caller, &p.project))
      .await?;
    Ok(Json(ProjectStatusOutput {
      constitution_version: status.constitution_version,
      specs: status.specs.into_iter().map(SpecOutput::from).collect(),
      active: status.active.map(ActiveStatusOutput::from),
    }))
  }

  #[tool(
    description = "List the revisions of a spec document or of the constitution, newest first"
  )]
  async fn list_revisions(
    &self,
    Parameters(p): Parameters<ListRevisionsParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<RevisionsOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let kind = parse_kind(&p.kind)?;
    let revisions = self
      .run(move |store| store.list_revisions(&caller, &p.project, p.number, kind))
      .await?;
    Ok(Json(RevisionsOutput {
      revisions: revisions.into_iter().map(RevisionOutput::from).collect(),
    }))
  }

  #[tool(description = "Read the body of a specific revision by id")]
  async fn get_revision(
    &self,
    Parameters(p): Parameters<GetRevisionParams>,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<RevisionBodyOutput>, McpError> {
    let caller = self.caller(&ctx).await?;
    let revision = self
      .run(move |store| store.get_revision(&caller, &p.project, p.revision_id))
      .await?;
    Ok(Json(RevisionBodyOutput {
      body: revision.body,
      kind: revision.kind.as_str().to_string(),
      number: revision.number,
      revision: revision.meta.into(),
    }))
  }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SddVaultMcp {
  fn get_info(&self) -> ServerConfig {
    ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
      .with_server_info(Implementation::new("sdd-vault", env!("CARGO_PKG_VERSION")))
      .with_instructions(
        "Store of Spec Driven Development documents. Projects are addressed as <organization>/<project> and specs by number. \
         Read documents with get_document, the constitution with get_constitution and templates with get_template.",
      )
  }
}
