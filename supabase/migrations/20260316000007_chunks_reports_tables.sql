-- BM25 chunks + 384-dim semantic embeddings, replacing .sensei/chunks.json and .sensei/embeddings.json
-- One row per indexable unit: symbol (file:name) or doc section (file#heading)
CREATE TABLE IF NOT EXISTS sensei.chunks (
  repo_id      UUID    NOT NULL REFERENCES sensei.repos(id) ON DELETE CASCADE,
  id           TEXT    NOT NULL,          -- e.g. "src/auth.ts:login" or "docs/design.md#storage"
  file_path    TEXT    NOT NULL,
  chunk_type   TEXT    NOT NULL,          -- 'symbol' | 'doc'
  text         TEXT    NOT NULL,
  content_hash TEXT    NOT NULL,
  tf           JSONB   NOT NULL DEFAULT '{}',
  embedding    vector(384),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (repo_id, id)
);

CREATE INDEX IF NOT EXISTS idx_chunks_repo_file
  ON sensei.chunks(repo_id, file_path);

CREATE INDEX IF NOT EXISTS idx_chunks_embedding
  ON sensei.chunks USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100)
  WHERE embedding IS NOT NULL;

-- Telemetry / benchmark feedback, replacing .sensei/reports.db
CREATE TABLE IF NOT EXISTS sensei.reports (
  id        TEXT        PRIMARY KEY,
  timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
  payload   JSONB       NOT NULL
);
