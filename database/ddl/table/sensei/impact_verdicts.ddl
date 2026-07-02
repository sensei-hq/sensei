set search_path to sensei, extensions;

-- Impact log: one row per shipped change worth remembering, with the
-- verdict about whether the change actually delivered its intended value.
-- Populated manually by the user (via the project window Impact screen)
-- until an outcome-attribution heuristic is available. Session linkage is
-- optional — retrospective entries about work shipped before session
-- capture existed still need a home.
create table if not exists impact_verdicts (
  id           uuid            primary key default gen_random_uuid()
, project_id   uuid            not null references sensei.projects(id) on delete cascade
, session_id   uuid            references activity.sessions(id) on delete set null
, title        text            not null
, note         text
, verdict      impact_verdict  not null default 'pending'
, created_at   timestamptz     not null default now()
, decided_at   timestamptz
, check (
    (verdict = 'pending' and decided_at is null)
    or
    (verdict <> 'pending' and decided_at is not null)
  )
);

create index if not exists impact_verdicts_project_idx
    on impact_verdicts(project_id, created_at desc);

create index if not exists impact_verdicts_verdict_idx
    on impact_verdicts(project_id, verdict, created_at desc)
 where verdict <> 'pending';

create index if not exists impact_verdicts_session_idx
    on impact_verdicts(session_id)
 where session_id is not null;

comment on table impact_verdicts is
'Impact log — retrospective judgments about shipped changes. Users file
an entry per meaningful change (feature, refactor, incident response),
then verdict it later as success / mixed / failure once the effect has
had time to settle. Powers the Impact screen in the project window and
feeds the analyzer''s effectiveness signals. Session linkage is optional
so pre-capture history can be backfilled.';

comment on column impact_verdicts.id
     is 'Surrogate primary key (UUID).';
comment on column impact_verdicts.project_id
     is 'FK to projects — every impact entry belongs to exactly one project.';
comment on column impact_verdicts.session_id
     is 'Optional FK to the session that shipped the change. Null for retrospective entries.';
comment on column impact_verdicts.title
     is 'Short label — the "what" being logged.';
comment on column impact_verdicts.note
     is 'Optional context — why the change was worth logging or what to watch for.';
comment on column impact_verdicts.verdict
     is 'Outcome: pending (not yet judged), success (delivered as intended), mixed (delivered value but not clean), failure (regressed or missed the goal).';
comment on column impact_verdicts.created_at
     is 'When the entry was logged.';
comment on column impact_verdicts.decided_at
     is 'When the verdict was assigned. NULL while pending; enforced by CHECK constraint.';
