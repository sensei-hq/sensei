set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.workspace_libraries_resolved AS
SELECT
    l.*,
    pl.enabled,
    pl.props          AS workspace_props,
    pl.modified_at    AS associated_at,
    CASE
        WHEN pl.workspace_id IS NULL THEN 'global'
        ELSE 'workspace'
    END               AS scope,
    pl.workspace_id   AS scoped_workspace_id
FROM sensei.libraries l
JOIN sensei.library_enablement pl ON pl.library_id = l.id;