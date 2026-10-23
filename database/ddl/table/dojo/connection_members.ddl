set search_path to dojo, extensions;

-- A forge connection's last COMPLETE member set (#308 R5c, D45): the forge's
-- stable ids of everyone its last complete member read listed, and when. A
-- Microsoft sign-in cannot read Azure DevOps, so it is provisioned from this.
-- Written only by dojo.reconcile_unit_members (a complete read); a partial read
-- never replaces it. The system's alone (policies/dojo/connection_members.sql).
create table if not exists dojo.connection_members (
  connection_id uuid        not null references dojo.tenant_connections(id) on delete cascade
, subject       text        not null
, read_at       timestamptz not null
, primary key (connection_id, subject)
);

create index if not exists connection_members_subject_idx
    on dojo.connection_members(subject);

comment on table dojo.connection_members is
'A forge connection''s last complete member set (#308 R5c, D45): who the last complete member read of
the organization listed, by stable id. A sign-in that cannot read the forge (Sign in with Microsoft)
provisions the person into every organization dōjō whose set lists them. Replaced only by
dojo.reconcile_unit_members; deleted with its connection. service_role only.';
comment on column dojo.connection_members.connection_id
     is 'The connection whose organization was read (dojo.tenant_connections.id).';
comment on column dojo.connection_members.subject
     is 'A listed member''s stable forge id, as dojo.identities.subject holds it (for Azure DevOps, the Microsoft Entra object id).';
comment on column dojo.connection_members.read_at
     is 'When the complete read that listed them ran; the same for the whole set.';

alter table dojo.connection_members enable row level security;
