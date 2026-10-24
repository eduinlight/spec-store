diesel::table! {
  users (id) {
    id -> BigInt,
    identity_subject -> Text,
    handle -> Text,
    email -> Text,
    name -> Text,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
  }
}

diesel::table! {
  organizations (id) {
    id -> BigInt,
    slug -> Text,
    name -> Text,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  organization_members (organization_id, user_id) {
    organization_id -> BigInt,
    user_id -> BigInt,
    role -> Text,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  invitations (id) {
    id -> BigInt,
    organization_id -> BigInt,
    email -> Text,
    token_hash -> Text,
    inviter_id -> BigInt,
    language -> Text,
    state -> Text,
    expires_at -> Timestamptz,
    accepted_by -> Nullable<BigInt>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
  }
}

diesel::table! {
  projects (id) {
    id -> BigInt,
    organization_id -> BigInt,
    slug -> Text,
    name -> Text,
    repository_url -> Nullable<Text>,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  memberships (project_id, user_id) {
    project_id -> BigInt,
    user_id -> BigInt,
    role -> Text,
  }
}

diesel::table! {
  specs (id) {
    id -> BigInt,
    project_id -> BigInt,
    number -> BigInt,
    slug -> Text,
    status -> Text,
    branch -> Text,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  documents (id) {
    id -> BigInt,
    project_id -> BigInt,
    spec_id -> Nullable<BigInt>,
    kind -> Text,
    current_revision_id -> Nullable<BigInt>,
  }
}

diesel::table! {
  revisions (id) {
    id -> BigInt,
    document_id -> BigInt,
    body -> Text,
    version -> Nullable<Text>,
    author_id -> BigInt,
    message -> Text,
    created_at -> Timestamptz,
    client -> Nullable<Text>,
  }
}

diesel::table! {
  audit_log (id) {
    id -> BigInt,
    actor_id -> BigInt,
    organization_id -> Nullable<BigInt>,
    project_id -> Nullable<BigInt>,
    action -> Text,
    target -> Text,
    created_at -> Timestamptz,
    client -> Nullable<Text>,
    origin -> Nullable<Text>,
    result -> Text,
    detail -> Nullable<Text>,
  }
}

diesel::table! {
  browser_sessions (id) {
    id -> BigInt,
    secret_hash -> Text,
    user_id -> BigInt,
    identity_session_id -> Nullable<Text>,
    refresh_token_sealed -> Binary,
    id_token_sealed -> Binary,
    access_expires_at -> Timestamptz,
    is_operator -> Bool,
    created_at -> Timestamptz,
    ended_at -> Nullable<Timestamptz>,
  }
}

diesel::table! {
  login_attempts (id) {
    id -> BigInt,
    state_hash -> Text,
    pkce_verifier_sealed -> Binary,
    return_to -> Text,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  email_outbox (id) {
    id -> BigInt,
    kind -> Text,
    recipient -> Text,
    language -> Text,
    payload -> Text,
    attempts -> Integer,
    next_attempt_at -> Timestamptz,
    sent_at -> Nullable<Timestamptz>,
    failed_at -> Nullable<Timestamptz>,
    created_at -> Timestamptz,
  }
}

diesel::joinable!(organization_members -> organizations (organization_id));
diesel::joinable!(organization_members -> users (user_id));
diesel::joinable!(invitations -> organizations (organization_id));
diesel::joinable!(projects -> organizations (organization_id));
diesel::joinable!(memberships -> projects (project_id));
diesel::joinable!(memberships -> users (user_id));
diesel::joinable!(specs -> projects (project_id));
diesel::joinable!(documents -> projects (project_id));
diesel::joinable!(revisions -> documents (document_id));
diesel::joinable!(revisions -> users (author_id));
diesel::joinable!(audit_log -> users (actor_id));
diesel::joinable!(audit_log -> organizations (organization_id));
diesel::joinable!(audit_log -> projects (project_id));
diesel::joinable!(browser_sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
  users,
  organizations,
  organization_members,
  invitations,
  projects,
  memberships,
  specs,
  documents,
  revisions,
  audit_log,
  browser_sessions,
  login_attempts,
  email_outbox,
);

diesel::define_sql_function! {
  fn iso_utc(t: diesel::sql_types::Timestamptz) -> diesel::sql_types::Text;
}
