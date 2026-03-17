-- Move indexed content out of .sensei/ files and into repos table
ALTER TABLE sensei.repos
  ADD COLUMN IF NOT EXISTS doc_fingerprints   JSONB,
  ADD COLUMN IF NOT EXISTS last_indexed_commit TEXT,
  ADD COLUMN IF NOT EXISTS stack_md           TEXT,
  ADD COLUMN IF NOT EXISTS shortcuts_md       TEXT,
  ADD COLUMN IF NOT EXISTS llms_txt           TEXT;
