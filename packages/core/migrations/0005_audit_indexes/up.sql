CREATE INDEX audit_log_by_project_time ON audit_log (project_id, created_at DESC);
CREATE INDEX audit_log_by_organization_time ON audit_log (organization_id, created_at DESC);
CREATE INDEX audit_log_by_project_action ON audit_log (project_id, action);
