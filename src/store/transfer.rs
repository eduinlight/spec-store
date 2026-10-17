use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult};
use crate::store::Store;
use crate::tree::ProjectTree;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportOptions {
  pub constitution_version: Option<String>,
  pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSpec {
  pub branch: String,
  pub status: SpecStatus,
  pub kinds: Vec<DocumentKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
  pub constitution_version: Option<String>,
  pub specs: Vec<ImportedSpec>,
  pub status_fallbacks: Vec<String>,
  pub documents: usize,
}

impl Store {
  pub fn import_tree(
    &self,
    _caller: &Caller,
    _project: &str,
    _tree: &ProjectTree,
    _options: &ImportOptions,
  ) -> StoreResult<ImportReport> {
    Err(StoreError::Validation("not implemented".into()))
  }
}
