CREATE TABLE IF NOT EXISTS sensei.extension_projects (
  extension_id  uuid         NOT NULL REFERENCES sensei.extensions(id) ON DELETE CASCADE,
  project_id    uuid         REFERENCES sensei.projects(id) ON DELETE CASCADE,
  enabled       boolean      NOT NULL DEFAULT true,
  props         jsonb        NOT NULL DEFAULT '{}',
  modified_at   timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS extension_projects_global_uniq
    ON sensei.extension_projects(extension_id)
 WHERE project_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS extension_projects_scoped_uniq
    ON sensei.extension_projects(extension_id, project_id)
 WHERE project_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS extension_projects_project_id_idx
    ON sensei.extension_projects(project_id)
 WHERE enabled AND project_id IS NOT NULL;

COMMENT ON TABLE sensei.extension_projects IS
'Many-to-many: extensions (skills/commands/agents/hooks) associated with projects or global.
project_id NULL = global (active in every project).
project_id = X  = active only in that project.
extensions.scope column is a seeding hint only; this table is authoritative at runtime.';
