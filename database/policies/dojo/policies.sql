-- RLS for dojo.policies (#236 step 3; access matrix "Knowledge").
--
-- A tenant's governance policies: every member reads them, only an admin sets
-- them. The UPDATE grant includes tenant_id and scope_key because the route
-- upserts on (tenant_id, scope_key), and PostgREST's upsert sets every posted
-- column on conflict; the with-check keeps the row in a tenant the caller admins.
-- Covered by database/tests/dojo/knowledge_access.sql.
alter table dojo.policies enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists policies_service_only on dojo.policies;

grant select, delete on dojo.policies to authenticated;
grant insert (tenant_id, scope_key, attribution_default, confidentiality, retention_days, updated_at)
    on dojo.policies to authenticated;
grant update (tenant_id, scope_key, attribution_default, confidentiality, retention_days, updated_at)
    on dojo.policies to authenticated;

drop policy if exists policies_read_member on dojo.policies;
create policy policies_read_member on dojo.policies
    for select to authenticated
    using (dojo.is_member(tenant_id));

drop policy if exists policies_write_admin on dojo.policies;
create policy policies_write_admin on dojo.policies
    for insert to authenticated
    with check (dojo.tenant_role_at_least(tenant_id, 'admin'));

drop policy if exists policies_update_admin on dojo.policies;
create policy policies_update_admin on dojo.policies
    for update to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'))
    with check (dojo.tenant_role_at_least(tenant_id, 'admin'));

drop policy if exists policies_delete_admin on dojo.policies;
create policy policies_delete_admin on dojo.policies
    for delete to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'));
