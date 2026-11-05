set search_path to sensei, extensions;

-- What this machine last pushed of a tracker board to one dōjō tenant, per
-- persona and part (spec docs/spec/daemon/tracker-delivery-metrics.md D3; #352).
create table if not exists tracker_push_marks (
  tracker_id    uuid          not null references sensei.trackers(id) on delete cascade
, persona_id    uuid          not null references sensei.personas(id) on delete cascade
, tenant_id     uuid          not null
, part          text          not null check (part in ('facts', 'effort'))
, attempted_at  timestamptz   not null default now()
, hash          text
, pushed_at     timestamptz
, failed_hash   text
, failed_at     timestamptz
, error         text
, primary key (tracker_id, persona_id, tenant_id, part)
);

comment on table tracker_push_marks is
'The re-push watermark of a tracker board''s delivery part, per persona and dōjō tenant. Written BEFORE a push (attempted_at), so a push the dōjō committed but whose answer was lost still leaves a row a withdrawal can find; acknowledged (hash, pushed_at) on accepted or stale; a refusal records failed_hash, failed_at and error. A part is sent when its hash differs or its mark is older than 6 hours, unless that very hash was refused within the hour.';

comment on column tracker_push_marks.persona_id
     is 'The signed-in persona whose session pushed; a persona''s marks go with it.';
comment on column tracker_push_marks.tenant_id
     is 'The dōjō tenant (dojo.tenants.id) the part went to; a re-filed workspace in another tenant starts afresh.';
comment on column tracker_push_marks.part
     is 'facts (the board''s one-copy part) or effort (this person''s).';
comment on column tracker_push_marks.attempted_at
     is 'When a push of this part was last attempted, written before sending.';
comment on column tracker_push_marks.hash
     is 'sha256 of the canonical JSON of the part last acknowledged (accepted or stale); NULL until one is.';
comment on column tracker_push_marks.pushed_at
     is 'When the dōjō last acknowledged the part.';
comment on column tracker_push_marks.failed_hash
     is 'The hash of the part the dōjō last refused (4xx); not re-sent within the hour.';
comment on column tracker_push_marks.failed_at
     is 'When the dōjō last refused the part.';
comment on column tracker_push_marks.error
     is 'The dōjō''s refusal, as it answered.';
