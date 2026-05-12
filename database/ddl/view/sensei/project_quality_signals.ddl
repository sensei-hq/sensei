set search_path to sensei, activity, inference, extensions;

CREATE OR REPLACE VIEW sensei.project_quality_signals AS
WITH ftr AS (
  SELECT project_id
       , AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END) AS ftr_7d
    FROM activity.sessions
   WHERE started_at > now() - interval '7 days'
     AND outcome IS NOT NULL
   GROUP BY project_id
),
patterns AS (
  SELECT f.project_id
       , COUNT(*) FILTER (WHERE dp.lifecycle = 'rule')    AS rules_count
       , COUNT(*)                                          AS total_patterns
    FROM inference.detected_patterns dp
    JOIN sensei.folders f ON f.id = dp.folder_id
   WHERE NOT dp.is_anti_pattern
   GROUP BY f.project_id
),
drift AS (
  SELECT f.project_id
       , COUNT(*) FILTER (WHERE di.status != 'current') AS open_drift_count
    FROM inference.drift_items di
    JOIN sensei.folders f ON f.id = di.folder_id
   GROUP BY f.project_id
),
tests AS (
  SELECT f.project_id
       , AVG(
           CASE WHEN (e.data->>'total')::int > 0
                THEN (e.data->>'passed')::numeric / (e.data->>'total')::numeric
                ELSE NULL
           END
         ) AS test_pass_rate
    FROM activity.events e
    JOIN sensei.folders f ON f.id = e.folder_id
   WHERE e.event_type = 'test'
     AND e.created_at > now() - interval '7 days'
   GROUP BY f.project_id
)
SELECT p.id                                                AS project_id
     , COALESCE(ftr.ftr_7d, 0)                            AS ftr_7d
     , CASE WHEN pat.total_patterns > 0
            THEN ROUND(pat.rules_count::numeric / pat.total_patterns, 2)
            ELSE NULL
       END                                                 AS pattern_compliance
     , COALESCE(d.open_drift_count, 0)                     AS open_drift_count
     , t.test_pass_rate
  FROM sensei.projects p
  LEFT JOIN ftr      ON ftr.project_id = p.id
  LEFT JOIN patterns pat ON pat.project_id = p.id
  LEFT JOIN drift    d   ON d.project_id = p.id
  LEFT JOIN tests    t   ON t.project_id = p.id;

comment on view project_quality_signals is
'Four quality indicators per project for the observatory quality signals row.
- ftr_7d: first-try-right rate over last 7 days (0.0-1.0)
- pattern_compliance: ratio of enforced rules to total detected patterns (0.0-1.0)
- open_drift_count: number of drifted or broken doc-code references
- test_pass_rate: average test pass rate from test events (0.0-1.0, null if no test events)';
