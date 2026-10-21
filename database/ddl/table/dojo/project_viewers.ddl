set search_path to dojo, extensions;

-- The project viewer (#307 D8) — the only permission a project has: a read-only
-- managerial view of its metrics, dashboards and reports, aggregated or split by
-- repository, never per-person. Granted by the dōjō's admins, to a member or to an
-- outsider (a client, an executive) who belongs to no dōjō. Access to code and to
-- per-person data lives on repositories (D5, D6), not here. Written only through
-- dojo.grant_project_viewer / dojo.revoke_project_viewer.
create table if not exists project_viewers (
  project_id   uuid        not null references dojo.projects(id) on delete cascade
, principal_id uuid        not null references dojo.principals(id) on delete cascade
, granted_by   uuid        references dojo.principals(id) on delete set null
, granted_at   timestamptz not null default now()
, primary key (project_id, principal_id)
);

create index if not exists project_viewers_principal_idx on project_viewers(principal_id);

comment on table project_viewers is
'Read-only managerial viewers of a project (#307 D8): its metrics, dashboards and
reports, never per-person. Admin-granted; the viewer may be an outsider in no dōjō.';
