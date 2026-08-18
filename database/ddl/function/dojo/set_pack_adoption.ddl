set search_path to dojo, sensei, extensions;

-- Adopt (or drop) a global library pack for a USER-scoped namespace — the write
-- analog of the dojo.rule_pack_library view boundary. A view can't write, so the
-- API adopts through this SECURITY DEFINER function: it runs with the OWNER's
-- privileges (postgres) so it writes sensei.namespaces + sensei.rule_pack_adoptions
-- WITHOUT the shared sensei.* tables granting supabase roles (which the daemon's
-- plain Postgres has no roles to receive). The API is granted EXECUTE only.
--
-- A `/you` caller adopts into their USER-scoped namespace (scope_key 'user', slug =
-- the caller's user id — stable, one per user), created on first adopt. Only a
-- GLOBAL, active library pack is adoptable (never an org's private pack). Idempotent:
-- re-adopt is a no-op; drop removes just this user's adoption. Returns false when the
-- pack slug is unknown/unavailable (the route maps that to 404).
--
-- SECURITY DEFINER + pinned search_path (can't be hijacked by a caller-set path),
-- mirroring dojo.owns_membership.
create or replace function dojo.set_pack_adoption(
    p_pack_slug text,
    p_user_id   text,
    p_user_name text,
    p_adopt     boolean
)
    returns boolean
    language plpgsql
    security definer
    set search_path = dojo, sensei, extensions
as $$
declare
    v_pack_id uuid;
    v_version integer;
    v_ns_id   uuid;
begin
    select id, version into v_pack_id, v_version
      from sensei.rule_packs
     where slug = p_pack_slug and owner_namespace_id is null and status = 'active';
    if v_pack_id is null then
        return false;
    end if;

    if p_adopt then
        insert into sensei.namespaces (scope_key, slug, name)
        values ('user', p_user_id, coalesce(nullif(p_user_name, ''), 'Personal'))
        on conflict (scope_key, slug) do update set name = excluded.name
        returning id into v_ns_id;

        insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, adopted_by)
        values (v_pack_id, v_ns_id, v_version, p_user_id)
        on conflict (pack_id, namespace_id) do nothing;
    else
        delete from sensei.rule_pack_adoptions a
         using sensei.namespaces n
         where a.namespace_id = n.id
           and n.scope_key = 'user' and n.slug = p_user_id
           and a.pack_id = v_pack_id;
    end if;
    return true;
end;
$$;

-- Revoke from `authenticated` too (not just PUBLIC): CREATE OR REPLACE FUNCTION preserves
-- prior grants, so a stale `to authenticated` grant from an earlier deploy must be revoked
-- explicitly, or the anon/authenticated-callable-definer-RPC advisor persists.
revoke all on function dojo.set_pack_adoption(text, text, text, boolean) from public, authenticated;
-- service_role only: the dōjō /v1 route calls this via the service_role client (see
-- rulepacks-data.ts setPackAdoption). NOT granted to `authenticated` — no client calls it,
-- so it isn't exposed as an anon/authenticated-callable SECURITY DEFINER RPC (advisor 0016xx).
grant execute on function dojo.set_pack_adoption(text, text, text, boolean) to service_role;
