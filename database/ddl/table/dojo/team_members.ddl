set search_path to dojo, extensions;

-- Who is in a team (#307 D7) — a grouping for reports and bulk viewer grants,
-- never access. `role` is unused by access since repository roles (D5).
-- Written through dojo.add_team_member / remove_team_member (admins).
create table if not exists team_members (
  team_id      uuid        not null references dojo.teams(id) on delete cascade
, principal_id uuid        not null references dojo.principals(id) on delete cascade
, role         dojo.member_role not null default 'contributor'
, added_at     timestamptz not null default now()
, primary key (team_id, principal_id)
);

create index if not exists team_members_principal_idx on team_members(principal_id);

comment on table team_members is
'Who is in a team (#307 D7): a grouping for reports and bulk viewer grants. Grants
no access — repository roles do (D5). Keyed on principal_id so an account merge or
split does not orphan it. RLS in policies/dojo/teams.sql.';
