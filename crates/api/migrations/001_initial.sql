PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS goals (
  id TEXT PRIMARY KEY,
  parent_id TEXT REFERENCES goals(id),
  data TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  data TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS session_targets (
  session_id TEXT NOT NULL REFERENCES sessions(id),
  goal_id TEXT NOT NULL REFERENCES goals(id),
  role TEXT NOT NULL CHECK (role IN ('primary', 'secondary', 'review')),
  PRIMARY KEY (session_id, goal_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS one_primary_per_session
  ON session_targets(session_id) WHERE role = 'primary';
CREATE TABLE IF NOT EXISTS difficulties (
  id TEXT PRIMARY KEY,
  source_session_id TEXT UNIQUE REFERENCES sessions(id),
  data TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS experiences (
  id TEXT PRIMARY KEY,
  source_session_id TEXT UNIQUE REFERENCES sessions(id),
  data TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS operations (
  id TEXT PRIMARY KEY,
  request TEXT NOT NULL,
  response TEXT NOT NULL
);
