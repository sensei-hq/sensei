set search_path to dojo, extensions;

-- Register one repository the signed-in person's daemon found (#236; access
-- matrix "Projects and repositories": a member registers).
--
-- Maps the repository to the ONE tenant of the caller's that its forge org is
-- connected to, and files it there — or returns why not:
--   no_connection  the org is connected to no tenant at all
--   not_a_member   it is connected, but to no tenant the caller is in
--   ambiguous      its slug resolves to two of the caller's tenants (an org
--                  renamed upstream and its old name re-registered) — attaching
--                  code to the wrong governance boundary is worse than refusing
--
-- SECURITY DEFINER because telling the first two apart reads every tenant's
-- connections; what leaves is a reason code, never another tenant's id. The
-- caller parses (provider, org) from the key (repo-mapping.ts knows the forge
-- hosts); this refuses a key whose path does not carry that org, so a direct
-- call cannot file someone else's repository under the caller's tenant.
-- Idempotent: a second registration returns the existing row.
create or replace function dojo.register_repository(
    p_repo_key   text,
    p_provider   dojo.forge_provider,
    p_org        text,
    p_name       text,
    p_remote_url text
)
    returns table (tenant_id uuid, tenant_key text, repository_id uuid, reason text)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_key  text := lower(btrim(p_repo_key));
    v_org  text := lower(btrim(p_org));
    v_hits uuid[];
    v_mine uuid[];
    v_id   uuid;
begin
    if v_key = '' or v_org = ''
       or not (v_org = any (string_to_array(v_key, '/'))
               or split_part(v_key, '/', 1) = v_org || '.visualstudio.com') then
        raise exception 'repository key % does not carry the org %', p_repo_key, p_org
            using errcode = 'invalid_parameter_value';
    end if;

    select array_agg(distinct c.tenant_id) into v_hits
      from dojo.tenant_connections c
     where c.provider = p_provider
       and lower(c.external_slug) = v_org;
    if v_hits is null then
        return query select null::uuid, null::text, null::uuid, 'no_connection'::text;
        return;
    end if;

    select array_agg(h) into v_mine from unnest(v_hits) h where dojo.is_member(h);
    if v_mine is null then
        return query select null::uuid, null::text, null::uuid, 'not_a_member'::text;
        return;
    end if;
    if cardinality(v_mine) > 1 then
        return query select null::uuid, null::text, null::uuid, 'ambiguous'::text;
        return;
    end if;

    insert into dojo.repositories as r (tenant_id, repo_key, name, remote_url, provider)
    values (v_mine[1], v_key, p_name, p_remote_url, p_provider)
    on conflict on constraint repositories_tenant_id_repo_key_key do nothing
    returning r.id into v_id;
    if v_id is null then
        select r.id into v_id from dojo.repositories r where r.tenant_id = v_mine[1] and r.repo_key = v_key;
    end if;

    return query select v_mine[1], t.key, v_id, null::text from dojo.tenants t where t.id = v_mine[1];
end;
$$;

revoke all on function dojo.register_repository(text, dojo.forge_provider, text, text, text) from public;
grant execute on function dojo.register_repository(text, dojo.forge_provider, text, text, text) to authenticated;

comment on function dojo.register_repository(text, dojo.forge_provider, text, text, text) is
'Register one repository under the ONE tenant of the caller''s its forge org is
connected to, or say why not (no_connection | not_a_member | ambiguous). Reads
every tenant''s connections; returns only a reason, never another tenant''s id.
Refuses a key whose path does not carry the org (#236).';
