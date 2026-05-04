CREATE OR REPLACE VIEW sensei.project_extensions_resolved AS
SELECT
    e.*,
    ep.enabled,
    ep.props          AS project_props,
    ep.modified_at    AS associated_at,
    CASE
        WHEN ep.project_id IS NULL THEN 'global'
        ELSE 'project'
    END               AS scope,
    ep.project_id     AS scoped_project_id
FROM sensei.extensions e
JOIN sensei.extension_projects ep ON ep.extension_id = e.id;
