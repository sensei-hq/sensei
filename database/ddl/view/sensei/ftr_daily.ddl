set search_path to sensei, activity, extensions;

CREATE OR REPLACE VIEW sensei.ftr_daily AS
SELECT f.project_id
     , date_trunc('day', s.started_at)::date         AS day
     , AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)   AS ftr_rate
     , COUNT(*)                                        AS session_count
     , SUM(s.corrections)                              AS correction_count
     , AVG(s.turns)                                    AS avg_turns
  FROM activity.sessions s
  JOIN sensei.folders     f ON f.id = s.folder_id
 WHERE s.started_at > now() - interval '90 days'
   AND s.outcome IS NOT NULL
 GROUP BY f.project_id, date_trunc('day', s.started_at)::date;

comment on view ftr_daily is
'Daily FTR (First-Try-Right) rate per project.
Powers sparkline charts in observatory home and project headers.
- project_id NULL rows represent holistic (cross-project) aggregate
- ftr_rate: 0.0-1.0 proportion of sessions with zero corrections
- 90-day rolling window; older data dropped automatically
- Query with WHERE project_id = $1 for project sparkline
- Query with no filter + GROUP BY day for holistic sparkline';
