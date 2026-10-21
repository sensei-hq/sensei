-- RLS for dojo.project_viewers (#307 D8).
--
-- A viewer sees their own grants; the dōjō's admins see every grant on its
-- projects. Writes go through dojo.grant_project_viewer / revoke_project_viewer
-- (SECURITY DEFINER, admins only). Covered by database/tests/dojo/project_viewers.sql.
alter table dojo.project_viewers enable row level security;

grant select on dojo.project_viewers to authenticated;
revoke insert, update, delete on dojo.project_viewers from authenticated;

drop policy if exists project_viewers_select on dojo.project_viewers;
create policy project_viewers_select
    on dojo.project_viewers
    for select
    to authenticated
    using (
        principal_id = dojo.current_principal_id()
        or dojo.administers_project(project_id)
    );
