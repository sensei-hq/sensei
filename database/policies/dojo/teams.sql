-- RLS for dojo.teams and dojo.team_members (#307 A3, D7).
--
-- A team is a grouping, never access. The dōjō's members read its teams and who is
-- on them; writes go through dojo.create_team / rename_team / delete_team /
-- add_team_member / remove_team_member (SECURITY DEFINER, admins). Covered by
-- database/tests/dojo/teams_groupings.sql.
alter table dojo.teams enable row level security;
alter table dojo.team_members enable row level security;

grant select on dojo.teams, dojo.team_members to authenticated;
revoke insert, update, delete on dojo.teams, dojo.team_members from authenticated;

drop policy if exists teams_service_only on dojo.teams;
drop policy if exists teams_select on dojo.teams;
create policy teams_select on dojo.teams
    for select to authenticated
    using (dojo.is_member(tenant_id));

drop policy if exists team_members_service_only on dojo.team_members;
drop policy if exists team_members_select on dojo.team_members;
create policy team_members_select on dojo.team_members
    for select to authenticated
    using (dojo.team_tenant_member(team_id));
