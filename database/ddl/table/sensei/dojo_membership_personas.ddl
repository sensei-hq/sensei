set search_path to sensei, extensions;

-- Which persona each daemon-local Dōjō membership belongs to (#236 step 4,
-- docs/plan/2026-10-12-dojo-device-plane-as-person.md). The daemon's tenant
-- calls present THAT persona's session, so the dōjō's policies see the person.
--
-- Its own table rather than a column on dojo_memberships: an installed daemon
-- upgrades with `dbd apply`, which creates what is missing but never alters an
-- existing table — a new column would reach fresh installs only.
--
-- Recorded when the membership is matched against the persona's own list
-- (GET /v1/you/memberships): at join, and once for memberships made before this
-- existed. A membership with no row here has no known persona.
create table if not exists dojo_membership_personas (
  membership_id uuid        primary key references dojo_memberships(id) on delete cascade
, persona_id    uuid        not null references personas(id) on delete cascade
, bound_at      timestamptz not null default now()
);

create index if not exists dojo_membership_personas_persona_idx on dojo_membership_personas(persona_id);

comment on table dojo_membership_personas is
'The persona each local Dōjō membership belongs to — whose session the daemon
presents on that membership''s tenant calls (#236 step 4). One persona per
membership; gone with either side.';
comment on column dojo_membership_personas.membership_id
     is 'sensei.dojo_memberships.id — the service membership id (dojo.memberships.id).';
comment on column dojo_membership_personas.persona_id
     is 'sensei.personas.id — the person the dōjō lists this membership under (GET /v1/you/memberships).';
comment on column dojo_membership_personas.bound_at
     is 'When the daemon matched the membership to the persona.';
