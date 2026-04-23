-- Sessions — AI coding sessions captured by hooks.
-- Each session belongs to one repo (identified by CWD).

CREATE TABLE IF NOT EXISTS sensei.sessions (
  id           text        PRIMARY KEY,
  repo_id      text        NOT NULL REFERENCES sensei.repos(repo_id),
  task         text        NOT NULL DEFAULT '',
  started_at   timestamptz NOT NULL DEFAULT now(),
  completed_at timestamptz,
  outcome      text,                          -- completed, blocked, partial
  acp_id       text,                          -- claude-code, cursor, etc.
  turns        integer     NOT NULL DEFAULT 0,
  corrections  integer     NOT NULL DEFAULT 0,
  metadata     jsonb       NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_sessions_repo ON sensei.sessions(repo_id, started_at DESC);

COMMENT ON TABLE sensei.sessions IS 'AI coding sessions. FTR = sessions with 0 corrections.';
COMMENT ON COLUMN sensei.sessions.outcome IS 'completed, blocked, partial — set by checkpoint.';
