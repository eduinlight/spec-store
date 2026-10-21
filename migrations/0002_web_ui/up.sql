ALTER TABLE projects ADD COLUMN repository_url TEXT;

CREATE TABLE sessions (
  id INTEGER PRIMARY KEY,
  secret_hash TEXT NOT NULL UNIQUE,
  token_id INTEGER NOT NULL REFERENCES api_tokens(id),
  user_id INTEGER NOT NULL REFERENCES users(id),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  expires_at TEXT NOT NULL,
  ended_at TEXT
);
