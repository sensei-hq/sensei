CREATE OR REPLACE VIEW sensei.project_libraries_resolved AS
SELECT
    l.*,
    pl.enabled,
    pl.props          AS project_props,
    pl.modified_at    AS associated_at,
    CASE
        WHEN pl.project_id IS NULL THEN 'global'
        ELSE 'project'
    END               AS scope,
    pl.project_id     AS scoped_project_id
FROM sensei.libraries l
JOIN sensei.project_libraries pl ON pl.library_id = l.id;
