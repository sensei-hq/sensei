set search_path to dojo, extensions;

-- A person's role on one repository (#307 A2, D5) — where access lives. The forge
-- column mirrors the git provider's per-repo permission, refreshed by the mirror
-- job (dojo.mirror_repository_roles); a dōjō admin may override it, including to
-- no access, and the override survives the next mirror. The effective role is the
-- override when one is set, else the forge's (dojo.repo_role_at_least).
create table if not exists repository_roles (
  repository_id    uuid             not null references dojo.repositories(id) on delete cascade
, principal_id     uuid             not null references dojo.principals(id) on delete cascade
, forge_permission text
, forge_role       dojo.member_role
, mirrored_at      timestamptz
, overridden       boolean          not null default false
, override_role    dojo.member_role
, overridden_by    uuid             references dojo.principals(id) on delete set null
, overridden_at    timestamptz
, primary key (repository_id, principal_id)
);

create index if not exists repository_roles_principal_idx on repository_roles(principal_id);

comment on table repository_roles is
'A person''s role on a repository (#307 D5): mirrored from the forge, overridable by
a dōjō admin (the override wins and survives re-mirroring).';
comment on column repository_roles.forge_permission
     is 'What the forge reports (read, triage, write, maintain, admin); null when the forge no longer lists the person.';
comment on column repository_roles.overridden
     is 'An admin override is in force; override_role then holds the role, or null for no access.';
