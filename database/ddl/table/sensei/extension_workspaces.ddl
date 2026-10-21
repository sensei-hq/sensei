set search_path to sensei, extensions;

CREATE TABLE IF NOT EXISTS sensei.extension_workspaces (
  extension_id  uuid         NOT NULL REFERENCES sensei.extensions(id) ON DELETE CASCADE,
  workspace_id  uuid         REFERENCES sensei.workspaces(id) ON DELETE CASCADE,
  enabled       boolean      NOT NULL DEFAULT true,
  props         jsonb        NOT NULL DEFAULT '{}',
  modified_at   timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS extension_workspaces_global_uniq
    ON sensei.extension_workspaces(extension_id)
 WHERE workspace_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS extension_workspaces_scoped_uniq
    ON sensei.extension_workspaces(extension_id, workspace_id)
 WHERE workspace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS extension_workspaces_workspace_id_idx
    ON sensei.extension_workspaces(workspace_id)
 WHERE enabled AND workspace_id IS NOT NULL;

COMMENT ON TABLE sensei.extension_workspaces IS
'Many-to-many: extensions (skills/commands/agents/hooks) associated with workspaces or global.
workspace_id NULL = global (active in every workspace).
workspace_id = X  = active only in that workspace.
extensions.scope column is a seeding hint only; this table is authoritative at runtime.';