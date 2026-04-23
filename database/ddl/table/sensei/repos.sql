-- Repos — individual git repositories tracked by sensei.
-- Each repo belongs to at most one project.

CREATE TABLE IF NOT EXISTS sensei.repos (
  repo_id     text        PRIMARY KEY,
  name        text        NOT NULL,
  path        text        NOT NULL UNIQUE,
  remote_url  text,
  indexed_at  timestamptz,
  last_error  text,
  duplicate_of text       REFERENCES sensei.repos(repo_id),
  stack       text[]      NOT NULL DEFAULT '{}',
  libs        text[]      NOT NULL DEFAULT '{}',
  status      text        NOT NULL DEFAULT 'active',
  project_id  text        REFERENCES sensei.projects(id) ON DELETE SET NULL,
  role        text        NOT NULL DEFAULT 'unknown',
  label       text,
  metadata    jsonb       NOT NULL DEFAULT '{}',
  created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_repos_project ON sensei.repos(project_id);
CREATE INDEX IF NOT EXISTS idx_repos_status  ON sensei.repos(status);

COMMENT ON TABLE sensei.repos IS 'Individual git repositories. Discovered by scanning folders. Each belongs to at most one project.';
COMMENT ON COLUMN sensei.repos.metadata IS 'JSON: {icon, external_links, summary} from metadata scanner.';
COMMENT ON COLUMN sensei.repos.role IS 'Role within project: backend, frontend, library, docs, infra, unknown.';
