ALTER TABLE prd_schema RENAME TO prd_schema_v5;
CREATE TABLE prd_schema(version INTEGER NOT NULL CHECK(version=6));
INSERT INTO prd_schema VALUES(6);
DROP TABLE prd_schema_v5;
CREATE TABLE prd_import_previews(id TEXT PRIMARY KEY,data TEXT NOT NULL);
CREATE TABLE prd_import_rows(source_key TEXT PRIMARY KEY,data TEXT NOT NULL);
CREATE TABLE prd_import_batches(id TEXT PRIMARY KEY,data TEXT NOT NULL);
CREATE TABLE prd_upgrade(id TEXT PRIMARY KEY,data TEXT NOT NULL);
