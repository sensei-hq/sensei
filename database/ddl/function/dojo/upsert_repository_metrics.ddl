set search_path to dojo, extensions;

-- File a batch of repo-scoped metric rows for the signed-in person's daemon
-- (#236; access matrix "Metrics").
--
-- Every row must name a repository the caller may sync — their own verdict row
-- of dojo.all_my_repositories says sync_enabled — and the row's tenant is taken
-- from that verdict row, never from the input. The caller resolves the batch and
-- rejects what it can name (metrics-ingest.ts); a row that still fails here
-- means the batch is not what that side resolved, so the WHOLE batch is refused
-- (insufficient_privilege) and nothing is written.
--
-- Upserts on the table's seven-column key (NULLS NOT DISTINCT, so a null
-- commit_sha or principal matches). Two rows for one key in a batch merge, the
-- later winning — as the rows would if written one after another. Returns the
-- number of rows written.
--
-- SECURITY DEFINER: dojo.repository_metrics takes no session write.
create or replace function dojo.upsert_repository_metrics(p_rows jsonb)
    returns integer
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_principal uuid := dojo.current_principal_id();
    v_refused   integer;
    v_written   integer;
begin
    -- A scope is 'repo' or 'user'; anything else is the caller's bug, refused
    -- by name rather than filed under a guess.
    if exists (select 1 from jsonb_array_elements(p_rows) e
                where coalesce(e ->> 'scope', 'repo') not in ('repo', 'user')) then
        raise exception 'a metric row''s scope must be repo or user'
            using errcode = 'invalid_parameter_value';
    end if;

    select count(*) into v_refused
      from jsonb_array_elements(p_rows) e
     where not exists (
            select 1 from dojo.all_my_repositories v
             where v.principal_id = v_principal
               and v.sync_enabled
               and v.repository_id = (e ->> 'repository_id')::uuid);
    if v_refused > 0 then
        raise exception '% metric row(s) name a repository the caller may not sync', v_refused
            using errcode = 'insufficient_privilege';
    end if;

    insert into dojo.repository_metrics as rm
           (tenant_id, repository_id, metric_id, scope, principal_id, commit_sha,
            computed_on, grain, value, props, source)
    -- A user-scoped row belongs to the CALLER — the session's principal, never
    -- a principal (or email) named in the input (#240).
    select distinct on (x.metric_id, x.repository_id, x.scope, x.commit_sha, x.computed_on, x.grain)
           v.tenant_id, x.repository_id, x.metric_id, x.scope,
           case when x.scope = 'user' then v_principal end, x.commit_sha,
           x.computed_on, x.grain, x.value, x.props, x.source
      from (select (e ->> 'repository_id')::uuid               as repository_id
                 , (e ->> 'metric_id')::uuid                   as metric_id
                 , coalesce(e ->> 'scope', 'repo')             as scope
                 , e ->> 'commit_sha'                          as commit_sha
                 , (e ->> 'computed_on')::date                 as computed_on
                 , coalesce(e ->> 'grain', 'daily')            as grain
                 , (e ->> 'value')::numeric                    as value
                 , coalesce(e -> 'props', '{}'::jsonb)         as props
                 , coalesce(e ->> 'source', 'measured')        as source
                 , ord
              from jsonb_array_elements(p_rows) with ordinality as t(e, ord)) x
      join dojo.all_my_repositories v
        on v.principal_id = v_principal
       and v.sync_enabled
       and v.repository_id = x.repository_id
     order by x.metric_id, x.repository_id, x.scope, x.commit_sha, x.computed_on, x.grain, x.ord desc
    on conflict (metric_id, repository_id, scope, principal_id, commit_sha, computed_on, grain)
    do update set value = excluded.value, props = excluded.props, source = excluded.source, pushed_at = now();
    get diagnostics v_written = row_count;
    return v_written;
end;
$$;

revoke all on function dojo.upsert_repository_metrics(jsonb) from public;
grant execute on function dojo.upsert_repository_metrics(jsonb) to authenticated;

comment on function dojo.upsert_repository_metrics(jsonb) is
'File a batch of metric rows for the caller — repo-scoped, or user-scoped and
then the caller''s own (principal from the session, never the input; #240). Every row must name a
repository the caller may sync (their own verdict row); the tenant comes from
that row, never the input; one failing row refuses the whole batch
(insufficient_privilege). Upserts on the seven-column key; returns rows written
(#236).';
