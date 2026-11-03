-- props.authored_at on dojo.repository_metrics is an ISO-8601 instant with an
-- offset, or absent (F4/M2): the workspace rollup casts it, and one malformed
-- value would fail the whole read for every viewer without naming its row.
-- Enforced twice: refused by name at the write boundary (metrics-ingest.ts)
-- and by this CHECK, so no other writer can store one.
--
-- WHAT BREAKS THIS TEST: a malformed authored_at inserting cleanly, or a valid
-- one (Z, numeric offset, fractional) refused.
--
-- The CHECK lives in ddl/table/dojo/repository_metrics.ddl (canonical; the
-- coordinator deploys it at merge). Until then this file applies it when
-- missing, inside its own rolled-back transaction (R1 lane rule — no lane
-- converges the shared DB; the run.sh --apply item will supersede this guard).
begin;

do $$
begin
    if not exists (select 1 from pg_constraint where conname = 'repository_metrics_authored_at_iso') then
        alter table dojo.repository_metrics
            add constraint repository_metrics_authored_at_iso
            check (props->>'authored_at' is null
                   or props->>'authored_at' ~ '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$');
    end if;
end $$;

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000006b', 'organization/ztest-metrics-authored', 'organization',
   'ztest-metrics-authored', 'authored_at check fixture', 'x');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000d1', 'b0000000-0000-4000-8000-00000000006b',
   'github.com/ztest-metrics-authored/r', 'r', 'github');

do $$
declare
    m_id uuid;
    ok boolean;
    v text;
    i int;
    vals text[] := array['2026-10-06T14:00:00Z', '2026-10-06T14:00:00+00:00',
                         '2026-10-06T14:00:00.123456+00:00'];
begin
    select id into m_id from sensei.metrics order by key limit 1;

    -- Malformed values fail the CHECK: garbage, and a local time with no
    -- offset (the cast must never depend on the session TimeZone).
    foreach v in array array['not-a-time', '2026-10-06T14:00:00'] loop
        ok := false;
        begin
            insert into dojo.repository_metrics
                (tenant_id, repository_id, metric_id, scope, computed_on, value, props)
            values ('b0000000-0000-4000-8000-00000000006b', '90000000-0000-4000-8000-0000000000d1',
                    m_id, 'repo', current_date, 1, jsonb_build_object('authored_at', v));
        exception when check_violation then
            ok := true;
        end;
        if not ok then
            raise exception 'authored_at_check: % inserted cleanly and must be refused', v;
        end if;
    end loop;

    -- Valid values insert, one day each (the unique key spans the day): Z, a
    -- numeric offset, fractional seconds, and an absent key.
    for i in 1..3 loop
        insert into dojo.repository_metrics
            (tenant_id, repository_id, metric_id, scope, computed_on, value, props)
        values ('b0000000-0000-4000-8000-00000000006b', '90000000-0000-4000-8000-0000000000d1',
                m_id, 'repo', current_date - i, 1, jsonb_build_object('authored_at', vals[i]));
    end loop;
    insert into dojo.repository_metrics
        (tenant_id, repository_id, metric_id, scope, computed_on, value, props)
    values ('b0000000-0000-4000-8000-00000000006b', '90000000-0000-4000-8000-0000000000d1',
            m_id, 'repo', current_date - 4, 1, '{}');
end $$;

rollback;
