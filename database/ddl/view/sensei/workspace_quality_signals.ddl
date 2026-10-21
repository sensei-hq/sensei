set search_path to sensei, activity, inference, extensions;

-- Quality indicators per workspace. test_pass_rate previously came from
-- activity.events (type 'test'), which was never populated — that table is
-- being retired, so the column is kept (same shape, read by get_quality_signals)
-- but sourced as NULL until a real test signal is derived from assistant_events.
--
-- Folder -> workspace comes from `folder_workspaces` (never `folders.workspace_id`).
-- That view is multi-valued — a folder of a repository serving two workspaces
-- returns two rows — but each row carries a DIFFERENT workspace_id, and
-- (folder_id, workspace_id) is unique there (repositories_in_workspaces' PK), so the
-- `GROUP BY workspace_id` already in `patterns` and `drift` lands each pattern or
-- drift item once per owning workspace. No double count, no extra DISTINCT.
CREATE OR REPLACE VIEW sensei.workspace_quality_signals AS
WITH ftr AS (
  SELECT workspace_id
       , AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END) AS ftr_7d
    FROM activity.sessions
   WHERE started_at > now() - interval '7 days'
     AND outcome IS NOT NULL
   GROUP BY workspace_id
),
patterns AS (
  SELECT fp.workspace_id
       , COUNT(*) FILTER (WHERE dp.lifecycle = 'rule')    AS rules_count
       , COUNT(*)                                          AS total_patterns
    FROM inference.detected_patterns dp
    JOIN sensei.folder_workspaces fp ON fp.folder_id = dp.folder_id
   WHERE NOT dp.is_anti_pattern
   GROUP BY fp.workspace_id
),
drift AS (
  SELECT fp.workspace_id
       , COUNT(*) FILTER (WHERE di.status != 'current') AS open_drift_count
    FROM inference.drift_items di
    JOIN sensei.folder_workspaces fp ON fp.folder_id = di.folder_id
   GROUP BY fp.workspace_id
)
SELECT p.id                                                AS workspace_id
     , COALESCE(ftr.ftr_7d, 0)                            AS ftr_7d
     , CASE WHEN pat.total_patterns > 0
            THEN ROUND(pat.rules_count::numeric / pat.total_patterns, 2)
            ELSE NULL
       END                                                 AS pattern_compliance
     , COALESCE(d.open_drift_count, 0)                     AS open_drift_count
     , NULL::numeric                                       AS test_pass_rate
  FROM sensei.workspaces p
  LEFT JOIN ftr      ON ftr.workspace_id = p.id
  LEFT JOIN patterns pat ON pat.workspace_id = p.id
  LEFT JOIN drift    d   ON d.workspace_id = p.id;

comment on view workspace_quality_signals is
'Four quality indicators per workspace for the observatory quality signals row.
- ftr_7d: first-try-right rate over last 7 days (0.0-1.0)
- pattern_compliance: ratio of enforced rules to total detected patterns (0.0-1.0)
- open_drift_count: number of drifted or broken doc-code references
- test_pass_rate: NULL for now (the events test source was retired; to be
  re-derived from assistant_events test runs in a later iteration)';
