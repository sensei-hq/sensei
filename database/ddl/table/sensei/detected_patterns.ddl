-- Detected patterns — code patterns found during indexing.

CREATE TABLE IF NOT EXISTS sensei.detected_patterns (
  id             text        PRIMARY KEY,
  name           text        NOT NULL,
  pattern_type   text        NOT NULL,
  instance_count integer     NOT NULL DEFAULT 0,
  instances      jsonb       NOT NULL DEFAULT '[]',
  project        text        NOT NULL,
  detected_at    timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_patterns_project ON sensei.detected_patterns(project);

COMMENT ON TABLE sensei.detected_patterns IS 'Code patterns: follow (adapter, factory) and anti-patterns (duplication, god nodes).';
