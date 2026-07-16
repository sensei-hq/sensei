set search_path to activity, sensei, extensions;

create table if not exists run_events (
  id         bigserial      primary key
, run_id     uuid           not null references activity.runs(id) on delete cascade
, kind       run_event_kind not null
, phase      text
, feature    text
, detail     jsonb          not null default '{}'
, created_at timestamptz    not null default now()
);

create index if not exists run_events_run_idx on run_events(run_id, created_at desc);
create index if not exists run_events_kind_idx on run_events(kind, created_at desc);

comment on table run_events is
'Append-only cadence log for a run — the source of the filtered Relay feed AND the
audit trail. One row per logical event (feature shipped · gate raised · paused on
limit · resumed · crashed · housekeeping · …). Logical progress only — never diffs
or raw tool output (D10). bigserial: high write volume.';

comment on column run_events.detail
     is 'Structured, stripped event payload (e.g. {"reset_at": …} for paused_on_limit). No code, no diffs.';
