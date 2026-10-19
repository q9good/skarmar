-- Parsed source cells live only in the private application database. Preview
-- rows expire after an hour; committed raw rows remain available for audit.
CREATE TABLE IF NOT EXISTS import_previews (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL REFERENCES auth_users(id),
  filename TEXT NOT NULL,
  file_sha256 TEXT NOT NULL,
  expires_at INTEGER NOT NULL,
  data TEXT NOT NULL,
  summary TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS import_previews_owner ON import_previews(owner_user_id,expires_at);

CREATE TABLE IF NOT EXISTS import_batches (
  id TEXT PRIMARY KEY,
  preview_id TEXT NOT NULL UNIQUE,
  filename TEXT NOT NULL,
  file_sha256 TEXT NOT NULL,
  committed_at TEXT NOT NULL,
  result TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_source_rows (
  source_document TEXT NOT NULL,
  source_sheet TEXT NOT NULL,
  source_row_key TEXT NOT NULL,
  raw_sha256 TEXT NOT NULL,
  raw_json TEXT NOT NULL,
  target_kind TEXT NOT NULL,
  target_id TEXT,
  batch_id TEXT NOT NULL REFERENCES import_batches(id),
  PRIMARY KEY (source_document,source_sheet,source_row_key)
);
