-- Index errors — files that failed during indexing.

CREATE TABLE IF NOT EXISTS sensei.index_errors (
  id        serial      PRIMARY KEY,
  repo_id   text        NOT NULL REFERENCES sensei.repos(repo_id) ON DELETE CASCADE,
  file_path text        NOT NULL,
  error     text        NOT NULL,
  adapter   text,
  phase     text,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_index_errors_repo ON sensei.index_errors(repo_id);

COMMENT ON TABLE sensei.index_errors IS 'Files that failed to index. Persisted for debugging.';
