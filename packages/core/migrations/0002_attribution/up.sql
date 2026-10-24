ALTER TABLE revisions ADD COLUMN client text;
ALTER TABLE audit_log ADD COLUMN client text, ADD COLUMN origin text,
  ADD COLUMN result text NOT NULL DEFAULT 'ok' CHECK (result IN ('ok','denied','conflict')),
  ADD COLUMN detail text;
