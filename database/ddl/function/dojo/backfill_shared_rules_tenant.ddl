set search_path to dojo, extensions;

create or replace function dojo.backfill_shared_rules_tenant()
    returns table (owned int, unprovable int)
    language plpgsql
    set search_path = dojo, pg_temp
as $$
begin
    -- A rule's owner is the tenant of its EARLIEST publish — the tenant that
    -- first published it. A later publish of the same rule by another tenant was
    -- the cross-tenant overwrite this column exists to end, so it proves nothing.
    with first_publish as (
        select distinct on (a.target) a.target, a.tenant_id
          from dojo.audit_events a
         where a.action = 'publish'
         order by a.target, a.ts, a.id
    )
    update dojo.shared_rules r
       set tenant_id = f.tenant_id
      from first_publish f
     where r.tenant_id is null
       and f.target = r.id::text;

    return query
        select count(*) filter (where r.tenant_id is not null)::int,
               count(*) filter (where r.tenant_id is null)::int
          from dojo.shared_rules r;
end;
$$;

revoke all on function dojo.backfill_shared_rules_tenant() from public;

comment on function dojo.backfill_shared_rules_tenant() is
'One-time, idempotent (#236): gives every unowned shared rule the tenant of its
earliest `publish` audit event. A rule with none stays NULL — no provable owner,
so no guessed one (decision 2026-10-12: hidden until claimed). Returns the counts;
run at deploy and report `unprovable`. Owner-only: no role is granted it.';
