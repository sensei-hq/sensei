set search_path to dojo, sensei, extensions;

-- Register the namespace a published rule files under, returning its id (POST
-- rules, #236 step 4). `sensei` is not exposed through PostgREST
-- (supabase/config.toml), so the Worker reaches sensei.namespaces only through a
-- dojo function. The namespace registry is shared across tenants and is the
-- system's: the service role executes this, a session does not.
create or replace function dojo.register_rule_namespace(p_scope_key text, p_slug text, p_name text)
    returns uuid
    language sql
    volatile
    security definer
    set search_path = dojo, sensei, pg_temp
as $$
    insert into sensei.namespaces (scope_key, slug, name)
    values (p_scope_key, p_slug, p_name)
    on conflict (scope_key, slug) do update set name = excluded.name, modified_at = now()
    returning id;
$$;

revoke all on function dojo.register_rule_namespace(text, text, text) from public;
grant execute on function dojo.register_rule_namespace(text, text, text) to service_role;

comment on function dojo.register_rule_namespace(text, text, text) is
'Get-or-create the sensei.namespaces row a published rule files under, by
(scope_key, slug), refreshing its name; returns its id. Service role only — the
namespace registry is the system''s.';
