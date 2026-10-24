ALTER TABLE invitations
  ADD COLUMN project_id bigint REFERENCES projects(id),
  ADD COLUMN project_role text CHECK (project_role IN ('owner', 'editor', 'reader')),
  ADD CONSTRAINT invitations_project_role_with_project CHECK ((project_id IS NULL) = (project_role IS NULL));

CREATE INDEX invitations_by_project ON invitations(project_id, id DESC) WHERE project_id IS NOT NULL;

ALTER TABLE memberships ADD COLUMN created_at timestamptz NOT NULL DEFAULT now();

UPDATE memberships
   SET created_at = projects.created_at
  FROM projects
 WHERE projects.id = memberships.project_id;
