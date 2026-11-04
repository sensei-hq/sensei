set search_path to dojo, extensions;

-- A ticket of a board's kept copy (spec tracker-delivery-metrics D4; #352).
create table if not exists delivery_tickets (
  board_id          uuid         not null references dojo.delivery_boards(id) on delete cascade
, key               text         not null
, title             text
, status_category   text
, points            numeric
, sprints           text[]       not null
, issue_created_at  timestamptz
, primary key (board_id, key)
);

alter table delivery_tickets enable row level security;

comment on table delivery_tickets is
'The non-removed tickets of a board''s kept copy, replaced with it as a whole. The title is the only ticket free text that travels (B9, B12): no description, comment, assignee or URL. Written and read only through security-definer functions.';

comment on column delivery_tickets.key
     is 'The ticket key, e.g. AUTH-42.';
comment on column delivery_tickets.title
     is 'The ticket''s title (summary), for every workspace classification (B9); NULL when the tracker gave none, never a made-up one.';
comment on column delivery_tickets.status_category
     is 'new, indeterminate, done or undefined; NULL when unknown.';
comment on column delivery_tickets.points
     is 'Story points from the board''s estimation field; NULL when unset.';
comment on column delivery_tickets.sprints
     is 'External ids of every non-removed sprint that lists the ticket (its membership, not only the current one).';
comment on column delivery_tickets.issue_created_at
     is 'When the issue was created in the tracker; NULL when not yet read.';
