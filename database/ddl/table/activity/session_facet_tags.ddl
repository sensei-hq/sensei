set search_path to activity, sensei, extensions;

-- The multiset halves of a session facet: which kinds of work it involved, and
-- which frictions showed up (spec 2026-08-26, D3).
--
-- Goal categories and friction kinds are the same SHAPE — {name → weight} per
-- session — so they share one table discriminated by `kind`. Two near-identical
-- tables would be the divergence risk the metrics consolidation already taught
-- us to avoid.
create table if not exists activity.session_facet_tags (
    session_id  text           not null                  -- activity.sessions.client_session_id
  , kind        facet_tag_kind not null                  -- goal | friction
  , value       text           not null                  -- e.g. 'feature_implementation', 'lost_context'
  , weight      integer        not null default 1
  , primary key (session_id, kind, value)
);

-- The retrospective's two biggest sections are group-bys over this table:
-- "what you work on" (kind='goal') and "where things go wrong" (kind='friction').
create index if not exists session_facet_tags_kind_value_idx
    on activity.session_facet_tags (kind, value);

comment on table activity.session_facet_tags is
'Goal categories and friction kinds per session, one row per tag (spec 2026-08-26).
One table for both multisets, discriminated by `kind`. Rewritten in place when a
session is re-analyzed.';
comment on column activity.session_facet_tags.value is
'Deliberately text, not an enum: the goal and friction vocabularies grow as more
ACPs are seen, and a new value should not need a migration. The analyzer discards
values outside its current closed list before writing, so a model inventing a
category cannot break a group-by. `kind` IS an enum because it is closed.';
comment on column activity.session_facet_tags.weight is
'How strongly the tag applies. 1 for a plain occurrence; higher when the analyzer
saw a session dominated by one kind of work.';
