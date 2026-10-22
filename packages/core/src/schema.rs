diesel::table! {
  users (id) {
    id -> BigInt,
    handle -> Text,
    is_admin -> BigInt,
    created_at -> Text,
  }
}

diesel::table! {
  api_tokens (id) {
    id -> BigInt,
    user_id -> BigInt,
    token_hash -> Text,
    label -> Text,
    created_at -> Text,
    last_used_at -> Nullable<Text>,
    revoked_at -> Nullable<Text>,
  }
}

diesel::table! {
  projects (id) {
    id -> BigInt,
    slug -> Text,
    name -> Text,
    created_at -> Text,
    repository_url -> Nullable<Text>,
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
    created_at -> Text,
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
    created_at -> Text,
  }
}

diesel::table! {
  audit_log (id) {
    id -> BigInt,
    actor_id -> BigInt,
    project_id -> Nullable<BigInt>,
    action -> Text,
    target -> Text,
    created_at -> Text,
  }
}

diesel::table! {
  sessions (id) {
    id -> BigInt,
    secret_hash -> Text,
    token_id -> BigInt,
    user_id -> BigInt,
    created_at -> Text,
    expires_at -> Text,
    ended_at -> Nullable<Text>,
  }
}

diesel::joinable!(api_tokens -> users (user_id));
diesel::joinable!(memberships -> projects (project_id));
diesel::joinable!(memberships -> users (user_id));
diesel::joinable!(specs -> projects (project_id));
diesel::joinable!(documents -> projects (project_id));
diesel::joinable!(documents -> specs (spec_id));
diesel::joinable!(revisions -> documents (document_id));
diesel::joinable!(revisions -> users (author_id));
diesel::joinable!(audit_log -> users (actor_id));
diesel::joinable!(audit_log -> projects (project_id));
diesel::joinable!(sessions -> api_tokens (token_id));
diesel::joinable!(sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
  users,
  api_tokens,
  projects,
  memberships,
  specs,
  documents,
  revisions,
  audit_log,
  sessions,
);
