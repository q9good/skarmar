ALTER TABLE prd_schema RENAME TO prd_schema_v4;
CREATE TABLE prd_schema(version INTEGER NOT NULL CHECK(version=5));
INSERT INTO prd_schema VALUES(5);
DROP TABLE prd_schema_v4;
CREATE TABLE prd_observations(id TEXT PRIMARY KEY,kind TEXT NOT NULL,unique_key TEXT NOT NULL,version INTEGER NOT NULL,data TEXT NOT NULL,UNIQUE(kind,unique_key));
CREATE TABLE prd_observation_revisions(id TEXT NOT NULL REFERENCES prd_observations(id),version INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(id,version));
