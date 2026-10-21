-- RLS for dojo.project_members (#307 A1).
--
-- Reads connect as `authenticated`: the members of the project's dōjō see who is
-- on its projects and in what role (the project Team tab). Writes go through
-- dojo.set_project_member / dojo.remove_project_member (SECURITY DEFINER).
-- Covered by database/tests/dojo/project_members.sql.
alter table dojo.project_members enable row level security;

grant select on dojo.project_members to authenticated;
revoke insert, update, delete on dojo.project_members from authenticated;

drop policy if exists project_members_select on dojo.project_members;
create policy project_members_select
    on dojo.project_members
    for select
    to authenticated
    -- SECURITY DEFINER, so it does not depend on dojo.projects' own RLS (which
    -- shows a project only to those who can read one of its repositories).
    using (dojo.project_role_at_least(project_id, 'contributor'));
