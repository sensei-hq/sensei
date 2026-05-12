set search_path to sensei, activity, extensions;

CREATE OR REPLACE VIEW sensei.project_hotspots AS
SELECT f.project_id
     , f.name                                          AS folder
     , COALESCE(e.data->>'file_path', e.data->>'module') AS file_path
     , COUNT(*) FILTER (WHERE e.event_type = 'edit')   AS edit_count
     , COUNT(*) FILTER (WHERE e.event_type = 'correction') AS correction_count
     , MAX(e.created_at)                               AS last_event_at
  FROM activity.events   e
  JOIN activity.sessions s ON s.id = e.session_id
  JOIN sensei.folders    f ON f.id = e.folder_id
 WHERE e.event_type IN ('edit', 'correction')
   AND COALESCE(e.data->>'file_path', e.data->>'module') IS NOT NULL
   AND e.created_at > now() - interval '30 days'
 GROUP BY f.project_id, f.name,
          COALESCE(e.data->>'file_path', e.data->>'module');

comment on view project_hotspots is
'Files with high edit/correction frequency per project.
Powers the Hotspots section in project overview and rework graph overlay.
- edit_count: how many edit events touched this file (from hooks)
- correction_count: how many correction events referenced this file/module
- Scoped to 30-day window
- Order by (correction_count + edit_count) DESC for rework ranking';
