set search_path to dojo, extensions;

-- A person's role on one project (#307 A1). It only RAISES what they may do
-- there — the effective role is the higher of the dōjō role and this one
-- (dojo.project_role_at_least) — and never lowers it. Admin is dōjō-wide and is
-- never a project role. Written only through dojo.set_project_member /
-- dojo.remove_project_member.
create table if not exists project_members (
  project_id   uuid             not null references dojo.projects(id) on delete cascade
, principal_id uuid             not null references dojo.principals(id) on delete cascade
, role         dojo.member_role not null
, added_by     uuid             references dojo.principals(id) on delete set null
, added_at     timestamptz      not null default now()
, updated_at   timestamptz      not null default now()
, primary key (project_id, principal_id)
, constraint project_members_not_admin check (role <> 'admin')
);

create index if not exists project_members_principal_idx on project_members(principal_id);

comment on table project_members is
'A person''s role on one project — raises what they may do there, never lowers
it (the effective role is the higher of the dōjō membership role and this). Admin
stays dōjō-wide. Written through dojo.set_project_member / remove_project_member.';
comment on column project_members.role
     is 'contributor, maintainer or lead on this project; never admin.';
