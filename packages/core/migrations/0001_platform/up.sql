CREATE FUNCTION iso_utc(t timestamptz) RETURNS text
  LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
  AS $$ SELECT to_char(t AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') $$;

CREATE TABLE users (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  identity_subject text NOT NULL UNIQUE,
  handle text NOT NULL UNIQUE,
  email text NOT NULL,
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE organizations (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug text NOT NULL UNIQUE,
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE organization_members (
  organization_id bigint NOT NULL REFERENCES organizations(id),
  user_id bigint NOT NULL REFERENCES users(id),
  role text NOT NULL CHECK (role IN ('owner', 'member')),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (organization_id, user_id)
);

CREATE TABLE invitations (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  organization_id bigint NOT NULL REFERENCES organizations(id),
  email text NOT NULL,
  token_hash text NOT NULL UNIQUE,
  inviter_id bigint NOT NULL REFERENCES users(id),
  language text NOT NULL DEFAULT 'en',
  state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'accepted', 'declined', 'revoked')),
  expires_at timestamptz NOT NULL,
  accepted_by bigint REFERENCES users(id),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX invitations_by_organization ON invitations(organization_id, id DESC);
CREATE INDEX invitations_by_email ON invitations(email) WHERE state = 'pending';

CREATE TABLE projects (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  organization_id bigint NOT NULL REFERENCES organizations(id),
  slug text NOT NULL,
  name text NOT NULL,
  repository_url text,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (organization_id, slug)
);

CREATE TABLE memberships (
  project_id bigint NOT NULL REFERENCES projects(id),
  user_id bigint NOT NULL REFERENCES users(id),
  role text NOT NULL CHECK (role IN ('owner', 'editor', 'reader')),
  PRIMARY KEY (project_id, user_id)
);

CREATE TABLE specs (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  project_id bigint NOT NULL REFERENCES projects(id),
  number bigint NOT NULL,
  slug text NOT NULL,
  status text NOT NULL CHECK (status IN ('draft', 'clarified', 'approved', 'archived')),
  branch text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (project_id, number),
  UNIQUE (project_id, slug)
);

CREATE TABLE documents (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  project_id bigint NOT NULL REFERENCES projects(id),
  spec_id bigint REFERENCES specs(id),
  kind text NOT NULL CHECK (kind IN ('constitution', 'spec', 'plan', 'tasks', 'research', 'validation')),
  current_revision_id bigint,
  CONSTRAINT documents_one_per_kind UNIQUE NULLS NOT DISTINCT (project_id, spec_id, kind)
);

CREATE TABLE revisions (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  document_id bigint NOT NULL REFERENCES documents(id),
  body text NOT NULL,
  version text,
  author_id bigint NOT NULL REFERENCES users(id),
  message text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX revisions_by_document ON revisions(document_id, id DESC);

ALTER TABLE documents
  ADD CONSTRAINT documents_current_revision_fk FOREIGN KEY (current_revision_id) REFERENCES revisions(id);

CREATE TABLE audit_log (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  actor_id bigint NOT NULL REFERENCES users(id),
  organization_id bigint REFERENCES organizations(id),
  project_id bigint REFERENCES projects(id),
  action text NOT NULL,
  target text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX audit_log_by_organization ON audit_log(organization_id, id DESC);
CREATE INDEX audit_log_by_project ON audit_log(project_id, id DESC);

CREATE TABLE browser_sessions (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  secret_hash text NOT NULL UNIQUE,
  user_id bigint NOT NULL REFERENCES users(id),
  identity_session_id text,
  refresh_token_sealed bytea NOT NULL,
  id_token_sealed bytea NOT NULL,
  access_expires_at timestamptz NOT NULL,
  is_operator boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now(),
  ended_at timestamptz
);

CREATE INDEX browser_sessions_by_identity_session ON browser_sessions(identity_session_id);

CREATE TABLE login_attempts (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  state_hash text NOT NULL UNIQUE,
  pkce_verifier_sealed bytea NOT NULL,
  return_to text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE email_outbox (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  kind text NOT NULL,
  recipient text NOT NULL,
  language text NOT NULL,
  payload text NOT NULL,
  attempts integer NOT NULL DEFAULT 0,
  next_attempt_at timestamptz NOT NULL DEFAULT now(),
  sent_at timestamptz,
  failed_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX email_outbox_due ON email_outbox(next_attempt_at) WHERE sent_at IS NULL AND failed_at IS NULL;
