set search_path to dojo, extensions;

-- Who is in a team — and therefore who can see the repositories its projects
-- hold. This is the authorization edge the metric RLS keys on.
--
-- GitHub's own per-repo access level (admin/write/read) is NOT used here. That
-- governs which repositories provisioning DISCOVERS; it must not decide what a
-- teammate sees inside a dōjō, or GitHub's ACL would silently define the
-- product's visibility rules.
create table if not exists team_members (
  team_id      uuid        not null references dojo.teams(id) on delete cascade
, principal_id uuid        not null references dojo.principals(id) on delete cascade
, role         dojo.member_role not null default 'contributor'
, added_at     timestamptz not null default now()
, primary key (team_id, principal_id)
);

create index if not exists team_members_principal_idx on team_members(principal_id);

comment on table team_members is
'Team membership — the single authorization path for repository metrics:
principal → team_members → team → projects → repositories.

Keyed on principal_id rather than an auth user id so an account merge or split
does not orphan access.';

alter table team_members enable row level security;
drop policy if exists team_members_service_only on team_members;
create policy team_members_service_only on team_members
    for all to authenticated, anon
    using (false) with check (false);
