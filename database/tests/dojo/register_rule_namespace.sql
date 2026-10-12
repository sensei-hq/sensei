-- The namespace a published rule files under (POST rules, #236 step 4).
--
-- `sensei` is not exposed through PostgREST (supabase/config.toml), so the Worker
-- reaches sensei.namespaces only through a dojo function. Registering a namespace
-- is the system's (the plan's route table): the service role may, a session may
-- not — a session naming namespaces into the shared registry is not a person's
-- write.
--
-- WHAT BREAKS THIS TEST: a function a session can execute; an upsert that mints
-- a second row for the same (scope_key, slug) or does not return the existing
-- id; a name that is not refreshed on re-registration; an unknown scope accepted.

begin;

-- Only the calls run as the service role: a deploy-built database grants it no
-- reach into `sensei`, so the function is its only way in. The checks below read
-- back as the owner.
set local role service_role;

do $$
begin
    perform set_config('ztest.first', dojo.register_rule_namespace('organization', 'ztest-rrn', 'Ztest')::text, true);
    perform set_config('ztest.again', dojo.register_rule_namespace('organization', 'ztest-rrn', 'Ztest renamed')::text, true);
    begin
        perform dojo.register_rule_namespace('no-such-scope', 'ztest-rrn', 'x');
        raise exception 'register_rule_namespace: accepted an unknown scope';
    exception when foreign_key_violation then null;
    end;
end $$;

reset role;

do $$
declare
    first uuid := nullif(current_setting('ztest.first', true), '')::uuid;
    again uuid := nullif(current_setting('ztest.again', true), '')::uuid;
    n     int;
begin
    if first is null or again is distinct from first then
        raise exception 'register_rule_namespace: re-registering returned % then % — expected one id', first, again;
    end if;
    select count(*) into n from sensei.namespaces where scope_key = 'organization' and slug = 'ztest-rrn';
    if n <> 1 then raise exception 'register_rule_namespace: % rows for one namespace', n; end if;
    if (select name from sensei.namespaces where id = first) <> 'Ztest renamed' then
        raise exception 'register_rule_namespace: the name was not refreshed';
    end if;
end $$;

set local role authenticated;

do $$
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000001a1"}', true);
    begin
        perform dojo.register_rule_namespace('organization', 'ztest-rrn-session', 'x');
        raise exception 'register_rule_namespace: a session registered a namespace';
    exception when insufficient_privilege then null;
    end;
end $$;

rollback;
