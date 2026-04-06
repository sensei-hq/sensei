set search_path to core, extensions;

create table if not exists team_members (
  team_id    uuid not null references core.teams(id) on delete cascade
, user_id    uuid not null references auth.users(id) on delete cascade
, role       text not null default 'member'
             check (role in ('maintainer','member'))
, joined_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
, primary key (team_id, user_id)
);

create index if not exists team_members_user_idx on team_members(user_id);

comment on table team_members is
'User membership in teams. Mirrors GitHub team roles.
- maintainer: can manage team membership and settings; implicit for account owners/admins
- member: read access to team resources
A user can be a maintainer in one team and a member in another.';

comment on column team_members.team_id is 'Foreign key to teams; identifies the team this membership belongs to.';
comment on column team_members.user_id is 'Foreign key to auth.users; identifies the user who is a member of the team.';
comment on column team_members.role is 'Team role: maintainer (can manage the team) or member (read access to team resources).';
comment on column team_members.joined_at is 'Timestamp when the user first joined the team.';
comment on column team_members.modified_at is 'Timestamp of the last modification to this row.';
comment on column team_members.modified_by is 'Identity (user, role, or service) that last modified this row.';
