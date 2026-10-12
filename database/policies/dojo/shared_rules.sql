-- dojo.shared_rules: owned by one tenant (#236, docs/spec/dojo/access-matrix.md
-- "Knowledge"). Members read their tenant's rules; maintainers and above publish,
-- retract and overwrite them. A rule with no owner (tenant_id NULL) matches no
-- policy, so it is visible to no one. No DELETE: a rule is retracted by
-- tombstone, never removed.
alter table dojo.shared_rules enable row level security;

drop policy if exists shared_rules_service_only on dojo.shared_rules;

grant select, insert, update on dojo.shared_rules to authenticated;
-- `seq` defaults to nextval: an insert without usage on it is refused before
-- any policy is consulted.
grant usage on sequence dojo.shared_rules_seq to authenticated;

drop policy if exists shared_rules_read_tenant on dojo.shared_rules;
create policy shared_rules_read_tenant
    on dojo.shared_rules
    for select
    to authenticated
    using (dojo.is_member(tenant_id));

drop policy if exists shared_rules_write_maintainer on dojo.shared_rules;
create policy shared_rules_write_maintainer
    on dojo.shared_rules
    for insert
    to authenticated
    with check (dojo.tenant_role_at_least(tenant_id, 'maintainer'));

drop policy if exists shared_rules_update_maintainer on dojo.shared_rules;
create policy shared_rules_update_maintainer
    on dojo.shared_rules
    for update
    to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'maintainer'))
    with check (dojo.tenant_role_at_least(tenant_id, 'maintainer'));
