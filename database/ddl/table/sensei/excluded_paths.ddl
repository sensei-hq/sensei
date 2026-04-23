-- Excluded paths — repos/folders the user has excluded from scanning.

CREATE TABLE IF NOT EXISTS sensei.excluded_paths (
  path        text        PRIMARY KEY,
  repo_id     text,
  reason      text,
  excluded_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE sensei.excluded_paths IS 'Paths excluded from scanning by user action.';
