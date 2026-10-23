-- RLS for dojo.forge_unit_mappings (#308 R5a).
--
-- A dōjō's admins read where the forge units filed into it come from. Nobody
-- writes the table directly: dojo.map_forge_unit (SECURITY DEFINER, service_role)
-- does, after the server has read the forge with the mapper's own token. Covered
-- by database/tests/dojo/forge_unit_mapping.sql.
alter table dojo.forge_unit_mappings enable row level security;

grant select on dojo.forge_unit_mappings to authenticated;
revoke insert, update, delete on dojo.forge_unit_mappings from authenticated;

drop policy if exists forge_unit_mappings_read_admin on dojo.forge_unit_mappings;
create policy forge_unit_mappings_read_admin on dojo.forge_unit_mappings
    for select to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'));
