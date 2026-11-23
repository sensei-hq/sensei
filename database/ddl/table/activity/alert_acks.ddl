set search_path to activity;

-- Alert dispositions: what the reader acked, and what cleared before.
--
-- The alerts themselves are never stored — they re-derive from the feeds
-- (executions, schedules, sync rows), which are already durable. This table
-- holds only what derivation cannot recompute: whether a standing alert is
-- acked, and how many times its condition cleared (recurrence memory for
-- escalation). A cleared condition sets acked=false and bumps seen_clears;
-- a recurrence reads seen_clears > 0 as escalated.
--
-- Lives in activity.* (not sensei.*): dispositions hang off execution
-- incidents and are meaningless without them, like capture_watermarks —
-- and activity never leaves the machine, which an ack must not do.
create table if not exists activity.alert_acks (
    class        text not null
  , alert_key    text not null
  , acked        boolean not null default false
  , seen_clears  integer not null default 0
  , updated_at   timestamptz not null default now()
  , primary key (class, alert_key)
);

comment on table activity.alert_acks is
'Operations alert dispositions: acked flags and recurrence memory per (class, key). Alerts re-derive from the feeds; this table holds only dispositions. Stale cleared rows prune in the PUT path.';

comment on column activity.alert_acks.seen_clears is 'How many times this condition stood and cleared. A recurrence with seen_clears > 0 escalates one level.';
