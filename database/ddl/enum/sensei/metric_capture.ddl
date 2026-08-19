set search_path to sensei, extensions;

-- Whether a metric's presence for a (repository, user, day) AUTHORIZES the
-- activity-pruner to reclaim that day's raw session activity — the capture-before-
-- reclaim guard (invariant I20, added after the 164GB nodes-bloat data-loss
-- incident). ONLY session-derived metrics may authorize reclaim:
--   session  -> session_outcomes + autonomy: the delivery / behaviour signals
--               computed FROM the raw sessions. Once written, that day's sessions
--               are safely reclaimable.
--   git      -> churn / quality: computed from git history, independent of the
--               session activity — NEVER authorizes reclaim.
--   snapshot -> knowledge / tool / health + the split rework_density: point-in-time
--               snapshots, likewise NEVER authorize reclaim.
-- ORTHOGONAL to metric_cadence: a snapshot metric can be day-cadence yet must NOT
-- authorize capture. Keying the pruner guard on cadence='day' would widen the
-- capture set to snapshot/day metrics and reclaim sessions before their session-
-- derived metric lands — the exact data-loss class the guard exists to prevent.
create type metric_capture
    as enum ('git', 'session', 'snapshot');
