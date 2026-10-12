set search_path to dojo, extensions;

-- Switch a catalogue metric off (or back on) for one repository, as the
-- signed-in person (#236; access matrix "Metrics": metric activations are the
-- tenant's cost decision).
--
-- Decided from the caller's own verdict row of dojo.all_my_repositories, which
-- also says which tenant's ruling this is. Only where the caller holds that
-- decision: configurable_by_me AND (a personal tenant OR a tenant admin).
--   no row for the caller, or an unknown metric → no_data_found (404)
--   not theirs to decide                        → insufficient_privilege (403)
-- Off is a row; on is its absence (a delete that matches nothing is the correct
-- no-op). Idempotent either way.
--
-- SECURITY DEFINER: dojo.metric_activations takes no session write.
create or replace function dojo.set_metric_activation(p_repo_key text, p_metric_key text, p_enabled boolean)
    returns table (repo_key text, metric text, enabled boolean, tenant text)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
#variable_conflict use_column
declare
    v_principal uuid := dojo.current_principal_id();
    mine        dojo.all_my_repositories;
    v_metric    uuid;
begin
    select * into mine
      from dojo.all_my_repositories v
     where v.principal_id = v_principal
       and v.repo_key = p_repo_key;
    if not found then
        raise exception 'no repository % for this account', p_repo_key using errcode = 'no_data_found';
    end if;
    if not (mine.configurable_by_me and (mine.origin = 'personal' or mine.role = 'admin')) then
        raise exception 'you may not change metric activation for %', p_repo_key
            using errcode = 'insufficient_privilege';
    end if;
    select m.id into v_metric from sensei.metrics m where m.key = p_metric_key;
    if v_metric is null then
        raise exception 'unknown metric %', p_metric_key using errcode = 'no_data_found';
    end if;

    if p_enabled then
        delete from dojo.metric_activations a
         where a.tenant_id = mine.tenant_id
           and a.repository_id = mine.repository_id
           and a.metric_id = v_metric;
    else
        insert into dojo.metric_activations (tenant_id, repository_id, metric_id, enabled)
        values (mine.tenant_id, mine.repository_id, v_metric, false)
        on conflict (tenant_id, repository_id, metric_id) do nothing;
    end if;

    return query select mine.repo_key, p_metric_key, p_enabled, mine.tenant;
end;
$$;

revoke all on function dojo.set_metric_activation(text, text, boolean) from public;
grant execute on function dojo.set_metric_activation(text, text, boolean) to authenticated;

comment on function dojo.set_metric_activation(text, text, boolean) is
'Switch a catalogue metric off (or on) for one of the caller''s repositories,
where they hold the tenant''s cost decision (configurable_by_me and a personal
tenant or an admin). no_data_found for an unknown repository or metric;
insufficient_privilege when it is not theirs (#236).';
