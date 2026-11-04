-- props.authored_at on dojo.repository_metrics is an ISO-8601 instant with an
-- offset, or absent (F4/M2): the workspace rollup casts it, and one malformed
-- value would fail the whole read for every viewer without naming its row.
-- Enforced twice: refused by name at the write boundary (metrics-ingest.ts)
-- and by this CHECK, so no other writer can store one.
--
-- WHAT BREAKS THIS TEST: a malformed authored_at inserting cleanly, or a valid
-- one (Z, numeric offset, fractional) refused; or the DDL not re-applying
-- cleanly a second time.
--
-- The files under test are the DDL files themselves (L0 ruling M2-a): the live
-- table and its indexes are renamed aside inside this rolled-back transaction,
-- then ddl/table/dojo/repository_metrics.ddl is applied TWICE. A hand-copied
-- constraint would hide a broken file, and an `alter table … add constraint`
-- form fails the second apply. Rolls back, so the shared DB is untouched
-- (R1 lane rule).
begin;

alter table dojo.repository_metrics rename to repository_metrics__live;
alter index dojo.repository_metrics_pkey rename to repository_metrics__live_pkey;
alter index dojo.repository_metrics_metric_id_repository_id_scope_principal__key
    rename to repository_metrics__live_key;
alter index dojo.repository_metrics_lookup rename to repository_metrics__live_lookup;

\ir ../../ddl/function/dojo/is_iso_instant.ddl
\ir ../../ddl/table/dojo/repository_metrics.ddl
-- The second apply is the point: a deploy runs the file again.
\ir ../../ddl/table/dojo/repository_metrics.ddl

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
                         '2026-10-06T14:00:00.123456+00:00', '2026-10-06T24:00:00Z',
                         '2026-10-06T14:00:00+15:59'];
begin
    select id into m_id from sensei.metrics order by key limit 1;

    -- Malformed values fail the CHECK: garbage, a local time with no offset
    -- (the cast must never depend on the session TimeZone), calendar
    -- impossibilities the cast refuses, and displacements Postgres refuses
    -- outright (22009: anything past ±15:59 — is_iso_instant answers false
    -- rather than raising).
    foreach v in array array['not-a-time', '2026-10-06T14:00:00',
                             '2026-02-30T00:00:00Z', '2026-13-01T00:00:00Z',
                             '2026-10-06T14:00:00+16:00', '2026-10-06T14:00:00+14:60'] loop
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
    -- numeric offset, fractional seconds, 24:00 (the cast accepts midnight
    -- end-of-day, so the CHECK agrees with it — ingest stays stricter), the
    -- +15:59 boundary Postgres still accepts, and an absent key.
    for i in 1..5 loop
        insert into dojo.repository_metrics
            (tenant_id, repository_id, metric_id, scope, computed_on, value, props)
        values ('b0000000-0000-4000-8000-00000000006b', '90000000-0000-4000-8000-0000000000d1',
                m_id, 'repo', current_date - i, 1, jsonb_build_object('authored_at', vals[i]));
    end loop;
    insert into dojo.repository_metrics
        (tenant_id, repository_id, metric_id, scope, computed_on, value, props)
    values ('b0000000-0000-4000-8000-00000000006b', '90000000-0000-4000-8000-0000000000d1',
            m_id, 'repo', current_date - 6, 1, '{}');
end $$;

-- The helper's answer cannot depend on DateStyle: a non-ISO shape is false
-- under every setting (the shape test lives inside the function for this,
-- which is what makes immutable sound).
do $$
declare
    a boolean;
    b boolean;
begin
    set local datestyle to 'ISO, MDY';
    select dojo.is_iso_instant('13/01/2026 00:00+00') into a;
    set local datestyle to 'ISO, DMY';
    select dojo.is_iso_instant('13/01/2026 00:00+00') into b;
    reset datestyle;
    if a is not false or b is not false then
        raise exception 'is_iso_instant must be false for a non-ISO shape under any DateStyle, got MDY=%, DMY=%', a, b;
    end if;
end $$;

rollback;
