CREATE OR REPLACE VIEW sensei.project_ftr_metrics AS
SELECT
  s.project_id,
  COUNT(*) FILTER (WHERE s.started_at > now() - interval '7d')   AS sessions_7d,
  AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)
    FILTER (WHERE s.started_at > now() - interval '14d')          AS ftr_14d,
  AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)
    FILTER (WHERE s.started_at > now() - interval '28d'
              AND s.started_at <= now() - interval '14d')         AS ftr_14d_prev
FROM activity.sessions s
WHERE s.project_id IS NOT NULL
GROUP BY s.project_id;
