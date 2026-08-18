set search_path to dojo, sensei, extensions;

-- Which packs are adopted, per adopting namespace — the read side of adoption for the dōjō
-- API (the browse marks a pack "adopted" for the caller). A dojo-schema view over the sensei
-- adoption tables. The browse reads this SERVER-SIDE (service_role) and filters to the
-- caller's USER-scoped namespace (scope_key 'user', slug = caller user id) for their slugs.
--
-- SECURITY: `security_invoker = on` — runs as the calling role, not the owner (Supabase lint
-- security_definer_view). It is read only server-side as service_role (granted SELECT on the
-- sensei.* base tables; service_role bypasses RLS) and is NOT granted to `authenticated`, so
-- no client can read cross-namespace adoptions directly via PostgREST — the per-caller filter
-- lives in the server route. Explicit REVOKE below because CREATE OR REPLACE VIEW does not
-- reset existing grants on redeploy.
create or replace view dojo.pack_adoption
with (security_invoker = on)
as
select p.slug        as pack_slug
     , n.scope_key    as scope_key
     , n.slug         as namespace_slug
  from sensei.rule_pack_adoptions a
  join sensei.rule_packs p  on p.id = a.pack_id
  join sensei.namespaces n  on n.id = a.namespace_id;

comment on view dojo.pack_adoption is
'Adopted packs per adopting namespace (pack_slug × namespace scope_key/slug), a dojo-schema
view over sensei.rule_pack_adoptions. security_invoker = on; read only server-side as
service_role and NOT granted to authenticated, so cross-namespace adoptions are never exposed
to clients. The server route filters to the caller''s user-scoped namespace.';

revoke select on dojo.pack_adoption from authenticated;
