set search_path to sensei, extensions;

create table if not exists tags (
  tag                      text        primary key
, category                 text
, created_at               timestamptz not null default now()
);

comment on table tags is
'Controlled vocabulary for tags used across the system.
Tables that need tags carry a tags text[] column for quick access.
This table provides autocomplete and validation.
- category: optional grouping (e.g. "stack", "domain", "status")';

comment on column tags.tag
     is 'Tag text value — the controlled vocabulary entry.';
comment on column tags.category
     is 'Optional category for grouping tags (e.g. "stack", "domain", "status").';
comment on column tags.created_at
     is 'Timestamp when this tag was first created.';
