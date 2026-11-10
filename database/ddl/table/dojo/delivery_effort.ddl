set search_path to dojo, extensions;

-- One contributor's effort on one item of a board — the board as a whole, a sprint
-- or a ticket — as their daemon reports it (spec tracker-delivery-metrics D1, D4,
-- A2; #353).
create table if not exists delivery_effort (
  board_id        uuid         not null references dojo.delivery_boards(id) on delete cascade
, item_kind       text         not null check (item_kind in ('project', 'sprint', 'ticket'))
, item_ref        text         not null
, principal_id    uuid         not null references dojo.principals(id) on delete cascade
, source          uuid         not null
, repository_ids  uuid[]       not null check (cardinality(repository_ids) > 0)
, "values"        jsonb        not null check (jsonb_typeof("values") = 'object')
, reasons         jsonb        not null check (jsonb_typeof(reasons) = 'object')
, reported_at     timestamptz  not null default now()
, primary key (board_id, item_kind, item_ref, principal_id, source)
, check ((item_kind = 'project') = (item_ref = ''))
);

create index if not exists delivery_effort_principal_idx on delivery_effort(principal_id);

alter table delivery_effort enable row level security;

comment on table delivery_effort is
'Each contributor''s own effort on a board, per item and install, computed by their daemon over sessions provably theirs (#240). A push replaces all of that contributor''s rows from that install for the board (B13). Readers see merged values (dojo.delivery_effort_merged), restricted by B16; never written or read but through security-definer functions.';

comment on column delivery_effort.item_kind
     is 'project (the board as a whole; item_ref is empty), sprint (item_ref = the sprint''s external id) or ticket (item_ref = its key).';
comment on column delivery_effort.source
     is 'The install that reported the row (its local persona id, a random pseudonym): one person on two machines files two sets of rows, each replaced only by its own install''s push, and counts as one contributor.';
comment on column delivery_effort.repository_ids
     is 'The verified repositories whose sessions the row was computed from: who may see the row per person is decided over every one of them (D4).';
comment on column delivery_effort."values"
     is 'The D1 wire fields: numerators and counts, never a ratio; a null has its reason in reasons.';
comment on column delivery_effort.reasons
     is 'The sdlc_metrics reason code for every null in values, keyed by field.';
comment on column delivery_effort.reported_at
     is 'When the dōjō accepted the row (its own clock).';
