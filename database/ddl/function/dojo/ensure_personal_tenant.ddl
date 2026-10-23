set search_path to dojo, extensions;

-- A person's personal dōjō, created if absent: exactly one per person (#331, D1).
--
-- kavach posts the session once per Supabase auth event, so a magic-link landing
-- runs sign-in provisioning twice, milliseconds apart. A find-then-insert from
-- the server let both passes see "none yet"; the loser collided on the slug, found
-- no membership (the winner had not written it) and created `<slug>-2`. Here the
-- find and the create are one transaction, serialised per person by an advisory
-- lock, and the tenant and its admin membership are written together.
--
-- Found by the person's personal MEMBERSHIP, never by re-deriving the slug: a
-- forge handle is renameable. A slug another person holds escalates to `-2`,
-- `-3`… (two different people can both be `devuser`); it is never joined.
-- service_role only.
create or replace function dojo.ensure_personal_tenant(
    p_principal uuid,
    p_slug      text,
    p_name      text,
    p_auth      dojo.auth_method,
    p_url_base  text
)
    returns table (tenant_id uuid, key text, role text, created boolean)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_key  text;
    v_slug text;
    v_id   uuid;
begin
    if p_principal is null or coalesce(p_slug, '') = '' or p_auth is null or coalesce(p_url_base, '') = '' then
        raise exception 'a personal dōjō needs a person, a slug, a sign-in method and a url'
            using errcode = 'invalid_parameter_value';
    end if;

    perform pg_advisory_xact_lock(hashtext('dojo.personal:' || p_principal::text));

    return query
        select t.id, t.key, m.role::text, false
          from dojo.memberships m
          join dojo.tenants t on t.id = m.tenant_id
         where m.user_id = p_principal and t.origin = 'personal'
         order by t.created_at, t.id
         limit 1;
    if found then
        return;
    end if;

    for attempt in 1..20 loop
        v_slug := case when attempt = 1 then p_slug else p_slug || '-' || attempt end;
        v_key := 'personal/' || v_slug;
        insert into dojo.tenants (key, origin, slug, name, dojo_url, scope)
        values (v_key, 'personal', v_slug, p_name, p_url_base || '/' || v_key, 'private')
        on conflict on constraint tenants_key_unique do nothing
        returning id into v_id;
        if v_id is not null then
            insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role, source)
            values (v_id, p_principal, 'personal', p_auth, 'admin', 'provisioned');
            return query select v_id, v_key, 'admin'::text, true;
            return;
        end if;
    end loop;

    raise exception 'no free personal slug near "%"', p_slug using errcode = 'unique_violation';
end;
$$;

revoke all on function dojo.ensure_personal_tenant(uuid, text, text, dojo.auth_method, text) from public, anon, authenticated;
grant execute on function dojo.ensure_personal_tenant(uuid, text, text, dojo.auth_method, text) to service_role;

comment on function dojo.ensure_personal_tenant(uuid, text, text, dojo.auth_method, text) is
'Find-or-create a person''s one personal dōjō with its admin membership, atomically under a per-person advisory lock (#331, D1). service_role only.';
