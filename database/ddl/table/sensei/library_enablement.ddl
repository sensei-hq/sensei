set search_path to sensei, extensions;

CREATE TABLE IF NOT EXISTS sensei.library_enablement (
  library_id   uuid         NOT NULL REFERENCES sensei.libraries(id) ON DELETE CASCADE,
  workspace_id uuid         REFERENCES sensei.workspaces(id) ON DELETE CASCADE,
  enabled      boolean      NOT NULL DEFAULT true,
  props        jsonb        NOT NULL DEFAULT '{}',
  modified_at  timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS library_enablement_global_uniq
    ON sensei.library_enablement(library_id)
 WHERE workspace_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS library_enablement_scoped_uniq
    ON sensei.library_enablement(library_id, workspace_id)
 WHERE workspace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS library_enablement_workspace_id_idx
    ON sensei.library_enablement(workspace_id)
 WHERE enabled AND workspace_id IS NOT NULL;

COMMENT ON TABLE sensei.library_enablement IS
'Many-to-many: libraries associated with workspaces or marked global.
workspace_id NULL = global (available in every workspace).
workspace_id = X  = scoped to that workspace only.
Populated by daemon indexer from referenced_libraries; editable by user.';