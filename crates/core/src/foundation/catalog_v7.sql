ALTER TABLE prd_schema RENAME TO prd_schema_v6;
CREATE TABLE prd_schema(version INTEGER NOT NULL CHECK(version=7));
INSERT INTO prd_schema VALUES(7);
DROP TABLE prd_schema_v6;

CREATE TABLE prd_entities_v7 (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('goal','method','course','teacher','domain')),
    version INTEGER NOT NULL CHECK (version > 0),
    name TEXT NOT NULL,
    data TEXT NOT NULL
);
CREATE TABLE prd_revisions_v7 (
    entity_id TEXT NOT NULL REFERENCES prd_entities_v7(id),
    version INTEGER NOT NULL,
    operation_id TEXT NOT NULL UNIQUE REFERENCES prd_operations(id),
    data TEXT NOT NULL,
    PRIMARY KEY(entity_id,version)
);
CREATE TABLE prd_method_goals_v7 (
    method_id TEXT NOT NULL REFERENCES prd_entities_v7(id),
    goal_id TEXT NOT NULL REFERENCES prd_entities_v7(id),
    PRIMARY KEY(method_id,goal_id)
);
INSERT INTO prd_entities_v7 SELECT id,kind,version,name,data FROM prd_entities;
INSERT INTO prd_revisions_v7 SELECT entity_id,version,operation_id,data FROM prd_revisions;
INSERT INTO prd_method_goals_v7 SELECT method_id,goal_id FROM prd_method_goals;
DROP TABLE prd_method_goals;
DROP TABLE prd_revisions;
DROP TABLE prd_entities;
ALTER TABLE prd_entities_v7 RENAME TO prd_entities;
ALTER TABLE prd_revisions_v7 RENAME TO prd_revisions;
ALTER TABLE prd_method_goals_v7 RENAME TO prd_method_goals;
CREATE UNIQUE INDEX prd_catalog_names ON prd_entities(kind,name) WHERE kind IN ('course','teacher','domain');
