set search_path to dojo, extensions;

-- People invited into a dōjō AS VIEWERS (#307 D8, revised 2026-10-21) — a manager
-- or client who needs the read-only view of some projects but has no repository
-- access and is not a member. Being a dōjō viewer grants nothing by itself: an
-- admin then grants the view on selected projects (dojo.project_viewers). It is
-- deliberately NOT a membership, so no member check (dojo.is_member) admits them.
-- Written by the invite accept path (dojo.add_dojo_viewer, service_role) and
-- removed by an admin (dojo.remove_dojo_viewer).
create table if not exists dojo_viewers (
  tenant_id    uuid        not null references dojo.tenants(id) on delete cascade
, principal_id uuid        not null references dojo.principals(id) on delete cascade
, invited_by   uuid        references dojo.principals(id) on delete set null
, joined_at    timestamptz not null default now()
, primary key (tenant_id, principal_id)
);

create index if not exists dojo_viewers_principal_idx on dojo_viewers(principal_id);

comment on table dojo_viewers is
'People in a dōjō as viewers only (#307 D8): not members; an admin grants them the
read-only view of selected projects. RLS in policies/dojo/dojo_viewers.sql.';
