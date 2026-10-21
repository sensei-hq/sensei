-- RLS for dojo.repository_roles (#307 A2, D5).
--
-- A person sees their own repository roles; the dōjō's admins see every role on its
-- repositories. The forge mirror writes as service_role; overrides go through
-- dojo.set_repository_role_override / clear_repository_role_override (admins).
-- Covered by database/tests/dojo/repository_roles.sql.
alter table dojo.repository_roles enable row level security;

grant select on dojo.repository_roles to authenticated;
revoke insert, update, delete on dojo.repository_roles from authenticated;

drop policy if exists repository_roles_select on dojo.repository_roles;
create policy repository_roles_select
    on dojo.repository_roles
    for select
    to authenticated
    using (
        principal_id = dojo.current_principal_id()
        or dojo.administers_repository(repository_id)
    );
