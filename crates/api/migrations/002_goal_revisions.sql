CREATE TABLE IF NOT EXISTS goal_revisions (
  goal_id TEXT NOT NULL REFERENCES goals(id),
  version INTEGER NOT NULL,
  data TEXT NOT NULL,
  PRIMARY KEY (goal_id, version)
);
