set search_path to dojo, sensei, extensions;

-- Which packs are adopted, per adopting namespace — the read side of adoption for
-- the dōjō API (the browse marks a pack "adopted" for the caller). A dojo-schema
-- view over the sensei adoption tables (owner privileges, so no sensei grant), like
-- dojo.rule_pack_library. The browse filters this to the caller's USER-scoped
-- namespace (scope_key 'user', slug = caller user id) to get their adopted slugs.
create or replace view dojo.pack_adoption as
select p.slug        as pack_slug
     , n.scope_key    as scope_key
     , n.slug         as namespace_slug
  from sensei.rule_pack_adoptions a
  join sensei.rule_packs p  on p.id = a.pack_id
  join sensei.namespaces n  on n.id = a.namespace_id;

comment on view dojo.pack_adoption is
'Adopted packs per adopting namespace (pack_slug × namespace scope_key/slug), a
dojo-schema view over sensei.rule_pack_adoptions so the API reads it with the view
owner''s privileges — no grant on sensei.*. The browse filters to the caller''s
user-scoped namespace to mark packs adopted.';

grant select on dojo.pack_adoption to authenticated;
