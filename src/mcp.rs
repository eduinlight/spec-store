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

use crate::model::{Caller, DocumentKind, SpecStatus, StoreError};
use crate::store::Store;
use crate::store::documents::RevisionMeta;
use crate::store::specs::SpecSummary;
use crate::templates;

#[derive(Clone)]
pub struct SpecStoreMcp {
  store: Store,
  fixed_caller: Option<Caller>,
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
  #[schemars(description = "Project slug")]
  pub project: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct DocumentParams {
  #[schemars(description = "Project slug")]
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
  #[schemars(description = "Project slug")]
  pub project: String,
  #[schemars(description = "Spec slug: one to four lowercase words joined by hyphens")]
  pub slug: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct PutDocumentParams {
  #[schemars(description = "Project slug")]
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
  #[schemars(description = "Project slug")]
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
  #[schemars(description = "Project slug")]
  pub project: String,
  #[schemars(description = "Spec number")]
  pub number: i64,
  #[schemars(description = "Target status: draft, clarified, approved or archived")]
  pub status: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct MarkTaskParams {
  #[schemars(description = "Project slug")]
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
    StoreError::Database(error) => {
      tracing::error!(%error, "database error");
      McpError::internal_error("database error", Some(json!({"kind": "internal"})))
    }
  }
}

#[tool_router]
impl SpecStoreMcp {
  pub fn new(store: Store) -> Self {
    Self {
      store,
      fixed_caller: None,
      tool_router: Self::tool_router(),
    }
  }

  pub fn acting_for(store: Store, caller: Caller) -> Self {
    Self {
      store,
      fixed_caller: Some(caller),
      tool_router: Self::tool_router(),
    }
  }

  fn caller(&self, ctx: &RequestContext<RoleServer>) -> Result<Caller, McpError> {
    if let Some(caller) = &self.fixed_caller {
      return Ok(caller.clone());
    }
    ctx
      .extensions
      .get::<Parts>()
      .and_then(|parts| parts.extensions.get::<Caller>().cloned())
      .ok_or_else(|| to_mcp_error(StoreError::Unauthenticated))
  }

  #[tool(
    description = "List the projects the caller belongs to with their role; admins see every project"
  )]
  async fn list_projects(
    &self,
    ctx: RequestContext<RoleServer>,
  ) -> Result<Json<ProjectsOutput>, McpError> {
    let caller = self.caller(&ctx)?;
    let projects = self.store.list_projects(&caller).map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let specs = self
      .store
      .list_specs(&caller, &p.project)
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let active = self
      .store
      .active_spec(&caller, &p.project)
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let kind = parse_kind(&p.kind)?;
    let doc = self
      .store
      .get_document(&caller, &p.project, p.number, kind)
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let c = self
      .store
      .get_constitution(&caller, &p.project)
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let created = self
      .store
      .create_spec(&caller, &p.project, &p.slug)
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let kind = parse_kind(&p.kind)?;
    let revision_id = self
      .store
      .put_document(
        &caller,
        &p.project,
        p.number,
        kind,
        &p.body,
        &p.message,
        p.expected_revision,
      )
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let revision_id = self
      .store
      .put_constitution(
        &caller,
        &p.project,
        &p.body,
        &p.version,
        &p.message,
        p.expected_revision,
      )
      .map_err(to_mcp_error)?;
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
    let caller = self.caller(&ctx)?;
    let status: SpecStatus = p.status.parse().map_err(to_mcp_error)?;
    let status = self
      .store
      .set_spec_status(&caller, &p.project, p.number, status)
      .map_err(to_mcp_error)?;
    Ok(Json(StatusOutput {
      number: p.number,
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
    let caller = self.caller(&ctx)?;
    let marked = self
      .store
      .mark_task_done(&caller, &p.project, p.number, &p.task_id, &p.commit)
      .map_err(to_mcp_error)?;
    Ok(Json(MarkedTaskOutput {
      revision_id: marked.revision_id,
      line: marked.line,
    }))
  }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SpecStoreMcp {
  fn get_info(&self) -> ServerConfig {
    ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
      .with_server_info(Implementation::new("spec-store", env!("CARGO_PKG_VERSION")))
      .with_instructions(
        "Store of Spec Driven Development documents. Projects are addressed by slug and specs by number. \
         Read documents with get_document, the constitution with get_constitution and templates with get_template.",
      )
  }
}
