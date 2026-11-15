set search_path to dojo, extensions;

-- A sprint of a board's kept copy (spec tracker-delivery-metrics D4; #352).
create table if not exists delivery_sprints (
  board_id     uuid         not null references dojo.delivery_boards(id) on delete cascade
, external_id  text         not null
, seq          bigint       not null
, name         text         not null
, state        text         not null
, start_at     timestamptz
, end_at       timestamptz
, complete_at  timestamptz
, removed      boolean      not null
, sdlc         jsonb
, primary key (board_id, external_id)
);

alter table delivery_sprints enable row level security;

comment on table delivery_sprints is
'The sprints of a board''s kept copy, replaced with it as a whole. Written and read only through security-definer functions.';

comment on column delivery_sprints.external_id
     is 'The provider''s sprint id, as text.';
comment on column delivery_sprints.seq
     is 'The provider''s creation order (Jira: the numeric sprint id); orders sprints after their start date.';
comment on column delivery_sprints.state
     is 'future, active or closed.';
comment on column delivery_sprints.removed
     is 'The sprint was removed from the board; kept for the copy, never read.';
comment on column delivery_sprints.sdlc
     is 'The sprint''s SprintSdlc figures as the daemon computed them (tracker-sdlc-metrics S6), facts inside; NULL for a sprint with no replay (removed), never a made-up figure.';
