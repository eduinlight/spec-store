use axum::http::request::Parts;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Json;
use rmcp::model::{ErrorCode, Implementation, ServerCapabilities, ServerConfig};
use rmcp::service::RequestContext;
use rmcp::{
  ErrorData as McpError, RoleServer, ServerHandler, schemars, tool, tool_handler, tool_router,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{Caller, StoreError};
use crate::store::Store;

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
