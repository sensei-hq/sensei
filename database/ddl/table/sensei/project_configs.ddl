-- Project configs — parsed config files from repos (package.json, Cargo.toml, etc.)

CREATE TABLE IF NOT EXISTS sensei.project_configs (
  repo_id    text        NOT NULL REFERENCES sensei.repos(repo_id) ON DELETE CASCADE,
  file_path  text        NOT NULL,
  file_type  text        NOT NULL,
  parsed_data jsonb      NOT NULL,
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (repo_id, file_path)
);

COMMENT ON TABLE sensei.project_configs IS 'Parsed config files extracted during indexing. Used for stack detection.';
