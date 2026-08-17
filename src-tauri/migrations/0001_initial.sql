-- Minimal infrastructure migration. Its only purpose is to prove that the
-- migration system works: a new database can be created, this migration
-- applied, and data written/read through it, surviving application
-- restarts.
--
-- Domain tables (projects, captures, processes, recordings, ...) are
-- intentionally NOT created here — see TASK-005 and docs/architecture.md
-- ("Persistence" section) for where the real schema will be introduced.

CREATE TABLE IF NOT EXISTS app_metadata (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
