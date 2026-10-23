-- An Azure DevOps grant is a secret (#308 R5b, D42: the Microsoft Entra refresh
-- token of the admin who connected; access tokens are never stored): it lives in Supabase Vault,
-- and dojo.tenant_connections keeps only the Vault secret's id, the token's state
-- (ok | needs_new_token) and its expiry when known.
--
-- Writing, replacing, removing and reading it go through SECURITY DEFINER
-- functions only service_role may execute. Every write is audited — "token set /
-- replaced / removed" and who did it — and no audit row, connection row or view
-- ever carries the token.
--
-- WHAT BREAKS THIS TEST: anon or authenticated executing any of the functions;
-- the token in tenant_connections, audit_events or any view; a replace that
-- leaves a second Vault secret (or the old value); a remove that leaves the
-- secret; a write by someone who does not administer the dōjō; a write with no
-- audit row; a read of a token marked needs_new_token; a deleted connection that
-- strands its secret in Vault.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000b01', 'a1000000-0000-4000-8000-000000000b01', 'ztest-vault-admin'),
  ('a0000000-0000-4000-8000-000000000b02', 'a1000000-0000-4000-8000-000000000b02', 'ztest-vault-member');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-000000000b0a', 'organization/ztest-vault-org', 'organization', 'ztest-vault-org', 'Org', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000b0a', 'a0000000-0000-4000-8000-000000000b01', 'admin', 'employer', 'sso'),
  ('b0000000-0000-4000-8000-000000000b0a', 'a0000000-0000-4000-8000-000000000b02', 'contributor', 'employer', 'sso');
insert into dojo.tenant_connections (id, tenant_id, provider, external_id, external_slug, connected_by) values
  ('c0000000-0000-4000-8000-000000000b01', 'b0000000-0000-4000-8000-000000000b0a', 'azure_devops',
   'ztest-az-b01', 'ztest-vault-org', 'a0000000-0000-4000-8000-000000000b01');

-- anon and authenticated execute none of them.
do $$
declare
    fn text;
    r  text;
begin
    foreach fn in array array[
        'dojo.set_forge_token(uuid, dojo.forge_provider, text, text, timestamptz)',
        'dojo.remove_forge_token(uuid, dojo.forge_provider, text)',
        'dojo.forge_token(dojo.forge_provider, text)',
        'dojo.mark_forge_token_rejected(dojo.forge_provider, text, text)',
        'dojo.rotate_forge_token(dojo.forge_provider, text, text, text)'
    ] loop
        foreach r in array array['anon', 'authenticated', 'public'] loop
            if r = 'public' then
                if exists (select 1 from pg_proc p,
                                  aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) a
                            where p.oid = fn::regprocedure and a.grantee = 0 and a.privilege_type = 'EXECUTE') then
                    raise exception 'vault: PUBLIC may execute %', fn;
                end if;
            elsif has_function_privilege(r, fn, 'execute') then
                raise exception 'vault: % may execute %', r, fn;
            end if;
        end loop;
        if not has_function_privilege('service_role', fn, 'execute') then
            raise exception 'vault: service_role may not execute %', fn;
        end if;
    end loop;
end $$;

-- …and a signed-in caller who tries is refused by the database itself.
set local role authenticated;
do $$
begin
    perform dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01',
                                 'ztest-pat-NEVER-STORED', null);
    raise exception 'vault: a signed-in caller set a token';
exception when insufficient_privilege then null;
end $$;
do $$
begin
    perform dojo.forge_token('azure_devops', 'ztest-az-b01');
    raise exception 'vault: a signed-in caller read a token';
exception when insufficient_privilege then null;
end $$;
reset role;

set local role service_role;
do $$
declare
    v_secret uuid;
    v_action text;
begin
    -- Only an admin of the dōjō writes its token.
    begin
        perform dojo.set_forge_token('a0000000-0000-4000-8000-000000000b02', 'azure_devops', 'ztest-az-b01',
                                     'ztest-pat-FROM-A-MEMBER', null);
        raise exception 'vault: a contributor set the token';
    exception when insufficient_privilege then null;
    end;
    -- An empty token is no token.
    begin
        perform dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01', '  ', null);
        raise exception 'vault: an empty token was stored';
    exception when invalid_parameter_value then null;
    end;
    -- A connection nobody made has no token to set.
    begin
        perform dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-none',
                                     'ztest-pat-NOWHERE', null);
        raise exception 'vault: a token was set on a connection that does not exist';
    exception when no_data_found then null;
    end;

    select dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01',
                                'ztest-pat-FIRST-0001', '2026-12-31T00:00:00Z') into v_action;
    if v_action <> 'set' then
        raise exception 'vault: the first write should say set, said %', v_action;
    end if;
    select c.token_secret_id into v_secret from dojo.tenant_connections c where c.external_id = 'ztest-az-b01';
    if v_secret is null then
        raise exception 'vault: the connection does not name its Vault secret';
    end if;
    if (select decrypted_secret from vault.decrypted_secrets where id = v_secret) <> 'ztest-pat-FIRST-0001' then
        raise exception 'vault: the Vault secret does not hold the token';
    end if;
    if dojo.forge_token('azure_devops', 'ztest-az-b01') <> 'ztest-pat-FIRST-0001' then
        raise exception 'vault: forge_token did not read the token back';
    end if;
    if (select token_state::text from dojo.tenant_connections where external_id = 'ztest-az-b01') <> 'ok'
       or (select token_expires_at from dojo.tenant_connections where external_id = 'ztest-az-b01')
          <> '2026-12-31T00:00:00Z'::timestamptz then
        raise exception 'vault: state or expiry not recorded';
    end if;

    -- A rejected token stops reads until it is replaced — but only the token the
    -- rejection was ABOUT (compare-and-set, #308 R5b review 2): a sync still
    -- holding an older token must not mark a newer grant.
    if dojo.mark_forge_token_rejected('azure_devops', 'ztest-az-b01', 'ztest-pat-OLDER-0000') then
        raise exception 'vault: a rejection of an older token marked the current one';
    end if;
    if (select token_state::text from dojo.tenant_connections where external_id = 'ztest-az-b01') <> 'ok' then
        raise exception 'vault: a stale rejection changed the state';
    end if;
    if not dojo.mark_forge_token_rejected('azure_devops', 'ztest-az-b01', 'ztest-pat-FIRST-0001') then
        raise exception 'vault: a rejection of the current token did not mark it';
    end if;
    if (select token_state::text from dojo.tenant_connections where external_id = 'ztest-az-b01') <> 'needs_new_token' then
        raise exception 'vault: a rejected token was not marked needs_new_token';
    end if;
    begin
        perform dojo.forge_token('azure_devops', 'ztest-az-b01');
        raise exception 'vault: a token marked needs_new_token was still read';
    exception when object_not_in_prerequisite_state then null;
    end;

    -- Replacing keeps exactly one secret, holding the new value, and clears the mark.
    select dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01',
                                'ztest-pat-SECOND-0002', null) into v_action;
    if v_action <> 'replaced' then
        raise exception 'vault: the second write should say replaced, said %', v_action;
    end if;
    if (select count(*) from vault.secrets s
         where s.id = v_secret or s.name = 'dojo/forge_token/c0000000-0000-4000-8000-000000000b01') <> 1 then
        raise exception 'vault: a replace left % secrets', (select count(*) from vault.secrets s
            where s.name like 'dojo/forge_token/c0000000-0000-4000-8000-000000000b01%');
    end if;
    if (select count(*) from vault.decrypted_secrets where decrypted_secret like 'ztest-pat-%') <> 1 then
        raise exception 'vault: a replace left the old token in Vault';
    end if;
    if dojo.forge_token('azure_devops', 'ztest-az-b01') <> 'ztest-pat-SECOND-0002' then
        raise exception 'vault: the replaced token is not the one read';
    end if;
    if (select token_state::text from dojo.tenant_connections where external_id = 'ztest-az-b01') <> 'ok'
       or (select token_expires_at from dojo.tenant_connections where external_id = 'ztest-az-b01') is not null then
        raise exception 'vault: a replace did not reset the state and expiry';
    end if;
end $$;
reset role;

-- A refresh rotates the token (D42): the system stores the new one in place —
-- one secret, the new value, no audit row (it is not a person's act).
set local role service_role;
do $$
declare
    v_secret uuid;
    v_audits bigint;
begin
    select c.token_secret_id into v_secret from dojo.tenant_connections c where c.external_id = 'ztest-az-b01';
    select count(*) into v_audits from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-000000000b0a';
    -- A late rotate from a sync that held an OLDER token changes nothing (#308 R5b review 2).
    if dojo.rotate_forge_token('azure_devops', 'ztest-az-b01', 'ztest-pat-OLDER-0000', 'ztest-pat-STALE-0008') then
        raise exception 'rotate: a stale rotate replaced the current token';
    end if;
    if dojo.forge_token('azure_devops', 'ztest-az-b01') <> 'ztest-pat-SECOND-0002' then
        raise exception 'rotate: a stale rotate changed the stored token';
    end if;
    if not dojo.rotate_forge_token('azure_devops', 'ztest-az-b01', 'ztest-pat-SECOND-0002', 'ztest-pat-ROTATED-0009') then
        raise exception 'rotate: the current token was not rotated';
    end if;
    if (select token_secret_id from dojo.tenant_connections where external_id = 'ztest-az-b01') <> v_secret
       or (select count(*) from vault.decrypted_secrets where decrypted_secret like 'ztest-pat-%') <> 1 then
        raise exception 'rotate: expected the one secret, updated in place';
    end if;
    if dojo.forge_token('azure_devops', 'ztest-az-b01') <> 'ztest-pat-ROTATED-0009' then
        raise exception 'rotate: the rotated token is not the one read';
    end if;
    if (select count(*) from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-000000000b0a') <> v_audits then
        raise exception 'rotate: a rotation was audited';
    end if;
    begin
        perform dojo.rotate_forge_token('azure_devops', 'ztest-az-none', 'ztest-pat-X', 'ztest-pat-NOWHERE');
        raise exception 'rotate: a connection with no token was rotated';
    exception when no_data_found then null;
    end;
    begin
        perform dojo.rotate_forge_token('azure_devops', 'ztest-az-b01', 'ztest-pat-ROTATED-0009', ' ');
        raise exception 'rotate: a blank token was stored';
    exception when invalid_parameter_value then null;
    end;

    -- A rotate never touches the state: it cannot un-reject a dead grant.
    perform dojo.mark_forge_token_rejected('azure_devops', 'ztest-az-b01', 'ztest-pat-ROTATED-0009');
    perform dojo.rotate_forge_token('azure_devops', 'ztest-az-b01', 'ztest-pat-ROTATED-0009', 'ztest-pat-ROTATED-0010');
    if (select token_state::text from dojo.tenant_connections where external_id = 'ztest-az-b01') <> 'needs_new_token' then
        raise exception 'rotate: a rotate reset a rejected grant to ok';
    end if;
end $$;
reset role;

-- The reconnect race end to end: a sync reads token A, an admin reconnects
-- (set_forge_token stores B, state ok), then the sync's refresh of A is refused
-- and its rotate of A arrives late — B is neither marked nor overwritten.
set local role service_role;
do $$
begin
    perform dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01',
                                 'ztest-pat-RECONNECTED-B', null);
    perform dojo.mark_forge_token_rejected('azure_devops', 'ztest-az-b01', 'ztest-pat-ROTATED-0010');
    perform dojo.rotate_forge_token('azure_devops', 'ztest-az-b01', 'ztest-pat-ROTATED-0010', 'ztest-pat-LATE-A2');
    if (select token_state::text from dojo.tenant_connections where external_id = 'ztest-az-b01') <> 'ok'
       or dojo.forge_token('azure_devops', 'ztest-az-b01') <> 'ztest-pat-RECONNECTED-B' then
        raise exception 'race: a stale sync marked or overwrote a reconnected grant';
    end if;
end $$;
reset role;

-- The token is in no table row and no view: not tenant_connections, not
-- audit_events, not any view of any schema but Vault's own.
do $$
declare
    v record;
    n bigint;
begin
    if exists (select 1 from dojo.tenant_connections c where c::text like '%ztest-pat-%') then
        raise exception 'vault: the token is in tenant_connections';
    end if;
    if exists (select 1 from dojo.audit_events a where a::text like '%ztest-pat-%') then
        raise exception 'vault: the token is in audit_events';
    end if;
    for v in
        select table_schema, table_name from information_schema.views
         where table_schema not in ('vault', 'pg_catalog', 'information_schema')
           -- Statement statistics hold THIS FILE's literal SQL text. The server
           -- never writes a token as a literal: PostgREST sends an rpc's
           -- arguments as a bind parameter, which pg_stat_statements records as $1.
           and table_name not like 'pg_stat_statements%'
    loop
        begin
            execute format('select count(*) from %I.%I x where x::text like %L', v.table_schema, v.table_name, '%ztest-pat-%')
               into n;
        exception when others then
            n := 0; -- a view that cannot be read here (needs a session) holds no token either way
        end;
        if n > 0 then
            raise exception 'vault: the token is in view %.%', v.table_schema, v.table_name;
        end if;
    end loop;
end $$;

-- Each write is audited, by its actor, without the token.
do $$
begin
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-000000000b0a' and action = 'forge_token_set'
           and actor_id = 'a0000000-0000-4000-8000-000000000b01') <> 1 then
        raise exception 'vault: expected one forge_token_set audit row';
    end if;
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-000000000b0a' and action = 'forge_token_replaced'
           and actor_id = 'a0000000-0000-4000-8000-000000000b01') <> 2 then
        raise exception 'vault: expected two forge_token_replaced audit rows (a replace, a reconnect)';
    end if;
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-000000000b0a' and action = 'forge_token_rejected') <> 2 then
        raise exception 'vault: expected two forge_token_rejected audit rows (one per rejection of the current token)';
    end if;
end $$;

-- Removing deletes the secret and audits it.
set local role service_role;
do $$
declare
    v_secret uuid;
begin
    select c.token_secret_id into v_secret from dojo.tenant_connections c where c.external_id = 'ztest-az-b01';
    begin
        perform dojo.remove_forge_token('a0000000-0000-4000-8000-000000000b02', 'azure_devops', 'ztest-az-b01');
        raise exception 'vault: a contributor removed the token';
    exception when insufficient_privilege then null;
    end;
    if not dojo.remove_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01') then
        raise exception 'vault: remove said there was no token';
    end if;
    if exists (select 1 from vault.secrets where id = v_secret) then
        raise exception 'vault: remove left the secret in Vault';
    end if;
    if (select token_secret_id from dojo.tenant_connections where external_id = 'ztest-az-b01') is not null
       or (select token_state from dojo.tenant_connections where external_id = 'ztest-az-b01') is not null then
        raise exception 'vault: remove left the connection naming a token';
    end if;
    if dojo.remove_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01') then
        raise exception 'vault: a second remove said it removed something';
    end if;
    begin
        perform dojo.forge_token('azure_devops', 'ztest-az-b01');
        raise exception 'vault: a removed token was read';
    exception when no_data_found then null;
    end;
end $$;
reset role;

do $$
begin
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-000000000b0a' and action = 'forge_token_removed'
           and actor_id = 'a0000000-0000-4000-8000-000000000b01') <> 1 then
        raise exception 'vault: expected one forge_token_removed audit row';
    end if;
end $$;

-- A connection that goes (its dōjō deleted, or the row itself) takes its secret with it.
set local role service_role;
do $$
begin
    perform dojo.set_forge_token('a0000000-0000-4000-8000-000000000b01', 'azure_devops', 'ztest-az-b01',
                                 'ztest-pat-THIRD-0003', null);
end $$;
reset role;
do $$
declare
    v_secret uuid;
begin
    select c.token_secret_id into v_secret from dojo.tenant_connections c where c.external_id = 'ztest-az-b01';
    delete from dojo.tenant_connections where external_id = 'ztest-az-b01';
    if exists (select 1 from vault.secrets where id = v_secret) then
        raise exception 'vault: a deleted connection stranded its secret';
    end if;
end $$;

rollback;
