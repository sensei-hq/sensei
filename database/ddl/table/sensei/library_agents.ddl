set search_path to sensei, extensions;

-- Review/assistant agents a library provides, declared in its own sensei.library.json
-- (workstream D). Mirrors library_skills minus the generation fields (agents are always
-- manifest-declared in v1). A per-library agent reviews a consuming app's use of the
-- library before coding and verifies after. Unique on (library_id, name).
create table if not exists library_agents (
  id            uuid          primary key default gen_random_uuid()
, library_id    uuid          not null references sensei.libraries(id) on delete cascade
, name          text          not null
, focus         text          not null
, body          text
, source        text          not null default 'manifest'
, source_path   text
, version_range text
, modified_at   timestamptz   not null default now()
, unique(library_id, name)
);

create index if not exists library_agents_library_id_idx
    on library_agents(library_id);

comment on table library_agents is
'Review agents a library provides via its sensei.library.json manifest (workstream D).
Surfaced for any project depending on the library. Unique on (library_id, name).';
comment on column library_agents.library_id is 'FK to libraries — which library ships this agent.';
comment on column library_agents.name       is 'Agent name — unique within a library.';
comment on column library_agents.focus      is 'What the agent reviews (e.g. "config+palette+component review").';
comment on column library_agents.body       is 'The agent markdown, fetched from the manifest path at ingest so reads are self-contained.';
comment on column library_agents.source     is 'Provenance: manifest (declared in sensei.library.json).';
comment on column library_agents.source_path is 'Manifest-relative path the body was read from (provenance).';
comment on column library_agents.version_range is 'Semver range (from the manifest) this agent applies to. Free text in v1.';
comment on column library_agents.modified_at is 'Timestamp of the last modification to this row.';
