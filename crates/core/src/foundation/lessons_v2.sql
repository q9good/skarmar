ALTER TABLE prd_schema RENAME TO prd_schema_v1;
CREATE TABLE prd_schema(version INTEGER NOT NULL CHECK(version=2));
INSERT INTO prd_schema(version) VALUES(2);
DROP TABLE prd_schema_v1;
CREATE TABLE prd_lessons(id TEXT PRIMARY KEY, version INTEGER NOT NULL, data TEXT NOT NULL);
CREATE TABLE prd_lesson_revisions(
  lesson_id TEXT NOT NULL REFERENCES prd_lessons(id), version INTEGER NOT NULL,
  operation_id TEXT NOT NULL UNIQUE REFERENCES prd_operations(id), data TEXT NOT NULL,
  PRIMARY KEY(lesson_id,version)
);
CREATE TABLE prd_knowledge(id TEXT PRIMARY KEY, source_lesson_id TEXT NOT NULL REFERENCES prd_lessons(id), data TEXT NOT NULL);
CREATE TABLE prd_actions(id TEXT PRIMARY KEY, source_lesson_id TEXT NOT NULL REFERENCES prd_lessons(id), data TEXT NOT NULL);
CREATE TABLE prd_lesson_refs(
  lesson_id TEXT NOT NULL REFERENCES prd_lessons(id), source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL, PRIMARY KEY(lesson_id,source_kind,source_id)
);
