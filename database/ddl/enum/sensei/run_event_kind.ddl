set search_path to sensei, extensions;

-- Append-only cadence events on a run (activity.run_events) — the source of the
-- filtered Relay feed AND the audit trail. Logical progress only, never diffs.
-- dbd orders variants alphabetically — rank via CASE, never rely on declared order.
create type run_event_kind
    as enum (
      'phase_started', 'feature_started', 'feature_done', 'phase_done'
    , 'gate_raised', 'gate_passed', 'flagged'
    , 'committed', 'pushed', 'merged', 'bumped'
    , 'paused_on_limit', 'resumed', 'throttled'
    , 'stalled', 'crashed', 'recovered', 'housekeeping'
    , 'done', 'failed'
    );
