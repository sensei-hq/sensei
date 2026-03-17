-- Pattern usage tracking — one row per pattern applied per session
CREATE TABLE IF NOT EXISTS sensei.pattern_usages (
  id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  repo_id        UUID        REFERENCES sensei.repos(id) ON DELETE CASCADE,
  session_id     TEXT,
  pattern_name   TEXT        NOT NULL,
  applied_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  outcome        TEXT,        -- completed | blocked | partial (filled by checkpoint())
  files_modified TEXT[],      -- filled by checkpoint() via git diff
  ftr_score      FLOAT        -- filled by checkpoint()
);

CREATE INDEX IF NOT EXISTS idx_pattern_usages_repo_pattern
  ON sensei.pattern_usages(repo_id, pattern_name);

CREATE INDEX IF NOT EXISTS idx_pattern_usages_session
  ON sensei.pattern_usages(session_id)
  WHERE session_id IS NOT NULL;
