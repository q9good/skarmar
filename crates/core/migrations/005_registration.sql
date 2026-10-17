ALTER TABLE auth_users ADD COLUMN email TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS auth_users_email ON auth_users(email) WHERE email IS NOT NULL;

CREATE TABLE IF NOT EXISTS auth_registration_requests (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  email TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS auth_registration_requests_created
  ON auth_registration_requests(created_at, id);

CREATE TABLE IF NOT EXISTS auth_registration_limits (
  key TEXT PRIMARY KEY,
  attempts INTEGER NOT NULL,
  window_start INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS auth_registration_limits_window
  ON auth_registration_limits(window_start);
