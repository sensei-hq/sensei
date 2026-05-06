CREATE TABLE IF NOT EXISTS sensei.project_libraries (
  library_id   uuid         NOT NULL REFERENCES sensei.libraries(id) ON DELETE CASCADE,
  project_id   uuid         REFERENCES sensei.projects(id) ON DELETE CASCADE,
  enabled      boolean      NOT NULL DEFAULT true,
  props        jsonb        NOT NULL DEFAULT '{}',
  modified_at  timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS project_libraries_global_uniq
    ON sensei.project_libraries(library_id)
 WHERE project_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS project_libraries_scoped_uniq
    ON sensei.project_libraries(library_id, project_id)
 WHERE project_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS project_libraries_project_id_idx
    ON sensei.project_libraries(project_id)
 WHERE enabled AND project_id IS NOT NULL;

COMMENT ON TABLE sensei.project_libraries IS
'Many-to-many: libraries associated with projects or marked global.
project_id NULL = global (available in every project).
project_id = X  = scoped to that project only.
Populated by daemon indexer from referenced_libraries; editable by user.';
