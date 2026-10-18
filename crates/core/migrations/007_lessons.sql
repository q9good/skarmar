CREATE TABLE IF NOT EXISTS lessons (
  id TEXT PRIMARY KEY,
  business_date TEXT NOT NULL,
  data TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS lessons_business_date ON lessons(business_date DESC, id);
