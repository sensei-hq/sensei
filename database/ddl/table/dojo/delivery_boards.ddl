set search_path to dojo, extensions;

-- One tracker board of a workspace, as its contributors' daemons report it
-- (spec docs/spec/daemon/tracker-delivery-metrics.md D4; #352).
create table if not exists delivery_boards (
  id                     uuid         primary key default gen_random_uuid()
, tenant_id              uuid         not null references dojo.tenants(id)    on delete cascade
, workspace_id           uuid         not null references dojo.workspaces(id) on delete cascade
, board_key              text         not null check (board_key ~ '^[0-9a-f]{64}$')
, kind                   text         not null
, name                   text         not null
, reported_by            uuid         not null references dojo.principals(id) on delete cascade
, reported_at            timestamptz  not null default now()
, history_at             timestamptz
, pending_tickets        integer      not null check (pending_tickets >= 0)
, predictability         numeric
, predictability_reason  text
, unique (workspace_id, board_key)
);

create index if not exists delivery_boards_tenant_idx on delivery_boards(tenant_id);

alter table delivery_boards enable row level security;

comment on table delivery_boards is
'A tracker board of a workspace and the ONE copy of its Jira-derived facts the dōjō keeps: the report that is not lesser (complete beats incomplete, then the later history_at, then the later arrival) replaces the board row and all its sprints and tickets together. Holds no tracker connection — no URL, account, secret, field mapping or sync state (B11). Written and read only through security-definer functions (dojo.file_delivery_facts, dojo.workspace_delivery_*).';

comment on column delivery_boards.board_key
     is 'hex(sha256(kind + LF + base_url + LF + board_ref)) of the board, computed by each daemon from its canonical connection: the same board gives the same key on every install, and no URL travels.';
comment on column delivery_boards.kind
     is 'The tracker kind, e.g. jira.';
comment on column delivery_boards.name
     is 'The board''s display name as the provider shows it (B15).';
comment on column delivery_boards.reported_by
     is 'The principal whose daemon reported the copy kept.';
comment on column delivery_boards.reported_at
     is 'When the dōjō accepted the copy kept (its own clock).';
comment on column delivery_boards.history_at
     is 'When the reporting daemon last read the board''s ticket history; NULL when never. Orders copies of equal completeness.';
comment on column delivery_boards.pending_tickets
     is 'Tickets whose history the reporting daemon had still to read; 0 = complete.';
comment on column delivery_boards.predictability
     is 'Spread of commitment reliability over the board''s last six closed sprints (tracker-sdlc-metrics S6); NULL with predictability_reason.';
comment on column delivery_boards.predictability_reason
     is 'The sdlc_metrics reason code when predictability is NULL.';
