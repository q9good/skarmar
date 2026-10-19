CREATE TABLE lesson_options (
  kind TEXT NOT NULL CHECK (kind IN ('course', 'teacher')),
  value TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1,
  version INTEGER NOT NULL DEFAULT 1,
  PRIMARY KEY (kind, value)
);
INSERT OR IGNORE INTO lesson_options(kind, value)
SELECT 'course', trim(json_extract(data, '$.course')) FROM lessons
WHERE trim(json_extract(data, '$.course')) <> '';
INSERT OR IGNORE INTO lesson_options(kind, value)
SELECT 'teacher', trim(json_extract(data, '$.teacher')) FROM lessons
WHERE trim(json_extract(data, '$.teacher')) <> '';
