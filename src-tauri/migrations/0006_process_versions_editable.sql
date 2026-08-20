-- TASK-019: lets a user edit a ProcessVersion's content in place (fixing
-- wording, correcting a step) without that counting as a new AI
-- generation. This is a deliberate, considered addition to
-- 0005_process_versions.sql's original "no updated_at, no update
-- method, fully immutable" design -- see docs/architecture.md ("Process
-- editor") and DECISIONS.md for the full reasoning: SS5's rule ("AI-
-- generated process regeneration must not silently overwrite a previous
-- process version") is specifically about *regeneration*, not about a
-- user consciously editing and saving their own already-generated
-- draft. Regenerating (calling the AI again) still always INSERTs a
-- brand new row, exactly as before -- this migration only adds a
-- second, distinct way an existing row's `content` can legitimately
-- change: a user editing it.
--
-- `updated_at` is nullable at the schema level (SQLite's ADD COLUMN
-- can't attach a computed/backfilled default in one statement) but is
-- immediately backfilled below for every row this migration finds, and
-- every future INSERT (see repositories::process_version) always
-- supplies it explicitly -- it is never actually NULL in practice.
ALTER TABLE process_versions ADD COLUMN updated_at INTEGER;

UPDATE process_versions SET updated_at = created_at WHERE updated_at IS NULL;
