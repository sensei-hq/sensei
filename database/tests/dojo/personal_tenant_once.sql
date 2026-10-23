-- A person gets exactly one personal dōjō, however many sign-in passes race (#331).
--
-- kavach posts the session once per Supabase auth event, so a magic-link landing
-- runs sign-in provisioning twice, milliseconds apart. Find-then-insert from the
-- server let both passes see "none yet": the second collided on the slug, found
-- no membership (the first had not written it yet) and created `<slug>-2`.
-- dojo.ensure_personal_tenant does find-or-create in ONE transaction, serialised
-- per person by an advisory lock, and writes the tenant and its admin membership
-- together. service_role only.
--
-- WHAT BREAKS THIS TEST: a second call creating a second tenant; a tenant with no
-- membership; another person's slug being joined instead of escalated; no lock
-- held; a session executing the function.

begin;

do $$
begin
    if has_function_privilege('authenticated', 'dojo.ensure_personal_tenant(uuid, text, text, dojo.auth_method, text)', 'execute')
       or has_function_privilege('anon', 'dojo.ensure_personal_tenant(uuid, text, text, dojo.auth_method, text)', 'execute') then
        raise exception 'personal tenant: a session may create personal dōjōs';
    end if;
end $$;

-- Somebody else already holds the slug `ptoz`.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, scope)
values ('b0000000-0000-4000-8000-00000000f001', 'personal/ptoz', 'personal', 'ptoz', 'Other', 'https://d/personal/ptoz', 'private');

set local role service_role;
do $$
declare
    me  uuid := 'a0000000-0000-4000-8000-00000000f002';
    r1  record;
    r2  record;
    n   int;
begin
    select * into r1 from dojo.ensure_personal_tenant(me, 'ptoz', 'Me', 'sso', 'https://d');
    if r1.key <> 'personal/ptoz-2' or not r1.created or r1.role <> 'admin' then
        raise exception 'personal tenant: first pass gave % (created %, role %)', r1.key, r1.created, r1.role;
    end if;
    -- The lock is held for the rest of the transaction.
    if not exists (select 1 from pg_locks where locktype = 'advisory' and pid = pg_backend_pid()
                   and objid = (hashtext('dojo.personal:' || me::text)::bigint & 4294967295)::oid) then
        raise exception 'personal tenant: no per-person advisory lock was held';
    end if;

    select * into r2 from dojo.ensure_personal_tenant(me, 'ptoz', 'Me', 'sso', 'https://d');
    if r2.tenant_id <> r1.tenant_id or r2.created then
        raise exception 'personal tenant: a second pass made % (created %)', r2.key, r2.created;
    end if;

    select count(*) into n from dojo.memberships m join dojo.tenants t on t.id = m.tenant_id
     where m.user_id = me and t.origin = 'personal';
    if n <> 1 then raise exception 'personal tenant: % personal memberships, want 1', n; end if;
    if exists (select 1 from dojo.memberships where tenant_id = 'b0000000-0000-4000-8000-00000000f001' and user_id = me) then
        raise exception 'personal tenant: joined another person''s dōjō';
    end if;
end $$;

rollback;
