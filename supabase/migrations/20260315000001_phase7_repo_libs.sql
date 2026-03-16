-- supabase/migrations/20260315000001_phase7_repo_libs.sql
CREATE TABLE sensei.repo_libs (
  id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  repo_id             uuid NOT NULL REFERENCES sensei.repos(id) ON DELETE CASCADE,
  name                text NOT NULL,
  source_type         text NOT NULL CHECK (source_type IN ('llms.txt', 'http', 'local')),
  base_url            text,
  local_path          text,
  skill_path          text,
  skill_generated_at  timestamptz,
  created_at          timestamptz DEFAULT now(),
  UNIQUE(repo_id, name)
);

CREATE INDEX repo_libs_repo_id_idx ON sensei.repo_libs(repo_id);
