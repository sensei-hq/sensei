-- RLS for dojo.dojo_viewers (#307 D8).
--
-- A viewer sees their own rows; the dōjō's admins see its viewers. Writes go
-- through dojo.add_dojo_viewer (service_role, the invite accept) and
-- dojo.remove_dojo_viewer (admins). Covered by database/tests/dojo/workspace_viewers.sql.
alter table dojo.dojo_viewers enable row level security;

grant select on dojo.dojo_viewers to authenticated;
revoke insert, update, delete on dojo.dojo_viewers from authenticated;

drop policy if exists dojo_viewers_select on dojo.dojo_viewers;
create policy dojo_viewers_select on dojo.dojo_viewers
    for select to authenticated
    using (principal_id = dojo.current_principal_id() or dojo.tenant_role_at_least(tenant_id, 'admin'));
