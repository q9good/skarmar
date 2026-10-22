CREATE TABLE lesson_targets (
  lesson_id TEXT NOT NULL REFERENCES lessons(id),
  goal_id TEXT NOT NULL REFERENCES goals(id),
  role TEXT NOT NULL CHECK (role IN ('primary','secondary','review')),
  PRIMARY KEY(lesson_id,goal_id)
);
CREATE UNIQUE INDEX one_primary_per_lesson ON lesson_targets(lesson_id) WHERE role='primary';
CREATE UNIQUE INDEX one_difficulty_per_lesson ON difficulties(json_extract(data,'$.source_lesson_id'))
  WHERE json_extract(data,'$.source_lesson_id') IS NOT NULL;
ALTER TABLE lesson_options ADD COLUMN renamed_to TEXT;
