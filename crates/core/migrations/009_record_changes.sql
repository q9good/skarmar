-- Tombstones prevent late retries from recreating deleted records. Private
-- snapshots stay in database backups; the public state only exposes kind/id.
CREATE TABLE deleted_records (
  kind TEXT NOT NULL,
  id TEXT NOT NULL,
  version INTEGER NOT NULL,
  data TEXT NOT NULL,
  deleted_at TEXT NOT NULL,
  deleted_by TEXT NOT NULL,
  source_session_id TEXT,
  PRIMARY KEY(kind,id)
);
CREATE INDEX deleted_records_source ON deleted_records(kind,source_session_id);
CREATE TABLE session_revisions (
  session_id TEXT NOT NULL REFERENCES sessions(id),
  version INTEGER NOT NULL,
  data TEXT NOT NULL,
  PRIMARY KEY(session_id,version)
);
