CREATE TABLE prd_schema (version INTEGER NOT NULL CHECK (version=1));
INSERT INTO prd_schema(version) VALUES(1);
CREATE TABLE prd_entities (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('goal','method','course','teacher')),
    version INTEGER NOT NULL CHECK (version > 0),
    name TEXT NOT NULL,
    data TEXT NOT NULL
);
CREATE UNIQUE INDEX prd_catalog_names ON prd_entities(kind,name) WHERE kind IN ('course','teacher');
CREATE TABLE prd_operations (
    id TEXT PRIMARY KEY,
    actor_id TEXT NOT NULL,
    request TEXT NOT NULL,
    response TEXT NOT NULL
);
CREATE TABLE prd_revisions (
    entity_id TEXT NOT NULL REFERENCES prd_entities(id),
    version INTEGER NOT NULL,
    operation_id TEXT NOT NULL UNIQUE REFERENCES prd_operations(id),
    data TEXT NOT NULL,
    PRIMARY KEY(entity_id,version)
);
CREATE TABLE prd_method_goals (
    method_id TEXT NOT NULL REFERENCES prd_entities(id),
    goal_id TEXT NOT NULL REFERENCES prd_entities(id),
    PRIMARY KEY(method_id,goal_id)
);
