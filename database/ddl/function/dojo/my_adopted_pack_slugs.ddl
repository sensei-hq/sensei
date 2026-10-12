set search_path to dojo, sensei, extensions;

-- The library packs the signed-in person has adopted into their own USER-scoped
-- namespace (#236). dojo.pack_adoption spans every namespace and stays the
-- service role's; a session reads its own through this, pinned to
-- dojo.current_principal_id().
create or replace function dojo.my_adopted_pack_slugs()
    returns setof text
    language sql
    stable
    security definer
    set search_path = dojo, sensei, pg_temp
as $$
    select p.slug
      from sensei.rule_pack_adoptions a
      join sensei.rule_packs p on p.id = a.pack_id
      join sensei.namespaces n on n.id = a.namespace_id
     where n.scope_key = 'user'
       and n.slug = dojo.current_principal_id()::text;
$$;

revoke all on function dojo.my_adopted_pack_slugs() from public;
grant execute on function dojo.my_adopted_pack_slugs() to authenticated;

comment on function dojo.my_adopted_pack_slugs() is
'Slugs of the library packs the signed-in person adopted into their own user
namespace — the session''s read of its adoptions (#236).';
