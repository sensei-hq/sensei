CREATE OR REPLACE VIEW sensei.project_extensions_resolved AS
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
    -- project-level overrides (ep wins over e defaults)
    COALESCE(ep.enabled, e.enabled)         AS enabled,
    COALESCE(ep.props, e.props)             AS props,
    e.source,
    ep.props                                AS project_props,
    ep.modified_at                          AS associated_at,
    CASE
        WHEN ep.project_id IS NULL THEN 'global'
        ELSE 'project'
    END                                     AS scope,
    ep.project_id                           AS scoped_project_id
FROM sensei.extensions e
JOIN sensei.extension_projects ep ON ep.extension_id = e.id;
