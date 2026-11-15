-- API roles for the relay schema: reach (USAGE) for the callers, everything
-- for the system. No-op where relay is not deployed (partial dbd scopes).
-- Per-table SELECT goes to authenticated in database/policies/relay/*.sql;
-- service_role bypasses RLS by definition, so the extra ALL is belt and
-- suspenders for tooling that checks grants instead of bypass rights.
do $$
begin
    if not exists (select 1 from pg_namespace where nspname = 'relay') then
        raise notice 'relay schema not deployed in this scope — skipping relay api grants';
        return;
    end if;

    grant usage on schema relay to anon, authenticated, service_role;
    grant all on all tables in schema relay to service_role;
    grant usage, select on all sequences in schema relay to service_role;
    alter default privileges in schema relay grant all on tables to service_role;
    alter default privileges in schema relay grant usage, select on sequences to service_role;
end;
$$;
