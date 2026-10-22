set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.workspace_extensions_resolved AS
SELECT
    e.id,
    e.plugin_id,
    e.kind,
    e.name,
    e.version,
    e.description,
    e.content,
    e.icons,
    e.tags,
    e.revision,
    e.modified_by,
    -- workspace-level overrides (ep wins over e defaults)
    COALESCE(ep.enabled, e.enabled)         AS enabled,
    COALESCE(ep.props, e.props)             AS props,
    e.source,
    ep.props                                AS workspace_props,
    ep.modified_at                          AS associated_at,
    CASE
        WHEN ep.workspace_id IS NULL THEN 'global'
        ELSE 'workspace'
    END                                     AS scope,
    ep.workspace_id                         AS scoped_workspace_id
FROM sensei.extensions e
JOIN sensei.extension_workspaces ep ON ep.extension_id = e.id;