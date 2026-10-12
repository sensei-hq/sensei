set search_path to dojo, sensei, extensions;

-- Adopt (or drop) a global library pack for the signed-in person's USER-scoped
-- namespace — the write analog of the dojo.rule_pack_library view boundary. A
-- view can't write, so a session adopts through this SECURITY DEFINER function:
-- it runs with the OWNER's privileges (postgres) so it writes sensei.namespaces +
-- sensei.rule_pack_adoptions WITHOUT the shared sensei.* tables granting supabase
-- roles (which the daemon's plain Postgres has no roles to receive).
--
-- The namespace is the CALLER's (scope_key 'user', slug = their principal id —
-- stable, one per person), created on first adopt. The user is derived from the
-- session (dojo.current_principal_id), never taken as an argument: the earlier
-- signature took a user id and was safe only while the service role alone could
-- call it (#236). No session → insufficient_privilege. Only a GLOBAL, active
-- library pack is adoptable (never an org's private pack). Idempotent: re-adopt
-- is a no-op; drop removes just this person's adoption. Returns false when the
-- pack slug is unknown/unavailable (the route maps that to 404).
--
-- SECURITY DEFINER + pinned search_path (can't be hijacked by a caller-set path),
-- mirroring dojo.owns_membership.
drop function if exists dojo.set_pack_adoption(text, text, text, boolean);

create or replace function dojo.set_pack_adoption(
    p_pack_slug text,
    p_user_name text,
    p_adopt     boolean
)
    returns boolean
    language plpgsql
    security definer
    set search_path = dojo, sensei, extensions
as $$
declare
    v_user    text := dojo.current_principal_id()::text;
    v_pack_id uuid;
    v_version integer;
    v_ns_id   uuid;
begin
    if v_user is null then
        raise exception 'a signed-in session is required' using errcode = 'insufficient_privilege';
    end if;

    select id, version into v_pack_id, v_version
      from sensei.rule_packs
     where slug = p_pack_slug and owner_namespace_id is null and status = 'active';
    if v_pack_id is null then
        return false;
    end if;

    if p_adopt then
        insert into sensei.namespaces (scope_key, slug, name)
        values ('user', v_user, coalesce(nullif(p_user_name, ''), 'Personal'))
        on conflict (scope_key, slug) do update set name = excluded.name
        returning id into v_ns_id;

        insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, adopted_by)
        values (v_pack_id, v_ns_id, v_version, v_user)
        on conflict (pack_id, namespace_id) do nothing;
    else
        delete from sensei.rule_pack_adoptions a
         using sensei.namespaces n
         where a.namespace_id = n.id
           and n.scope_key = 'user' and n.slug = v_user
           and a.pack_id = v_pack_id;
    end if;
    return true;
end;
$$;

revoke all on function dojo.set_pack_adoption(text, text, boolean) from public;
grant execute on function dojo.set_pack_adoption(text, text, boolean) to authenticated, service_role;
