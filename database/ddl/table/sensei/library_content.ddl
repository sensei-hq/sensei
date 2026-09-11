set search_path to sensei, extensions;

-- EVERYTHING A LIBRARY VERSION PROVIDES — skills, agents and documentation
-- pages in one table, discriminated by `kind` (R12).
--
-- Replaces `library_skills`, `library_agents` and `library_pages`. Those three
-- shared nine of their columns (library_version_id, name, body, source,
-- source_path, version_range, scope, origin, modified_at) and differed in a
-- handful; agents were literally documented as "mirrors library_skills minus
-- the generation fields". The cost of that shape was structural: a fifth kind
-- of content meant a fourth table, and "everything about library X" was a
-- three-arm UNION that a new kind silently fell out of.
--
-- Keyed to a VERSION, not a library (S10 option B). A library's skills can
-- differ between releases, and content hanging off the identity could not say
-- which release it described.
--
-- The kind-specific columns are nullable and that is deliberate — NOT a
-- modelling accident. Each is meaningful for exactly one kind, listed below
-- with its owner, so a reader never has to guess whether a null is "absent"
-- or "not applicable":
--
--   focus, version_range   skill + agent
--   tokens, generated_at   skill only (null until skill-gen runs)
--   url, local_path        page only — where it was fetched from
--   description, component page only
--   source_type            page only — the fetch route (llms.txt|http|local)
--   embedding              page only — 768-dim, pages are what gets searched
create table if not exists library_content (
  id                       uuid        primary key default gen_random_uuid()
, library_version_id       uuid        not null references sensei.library_versions(id) on delete cascade
, kind                     library_content_kind not null
  -- `title` for a page, `name` for a skill or agent. One concept, one column.
, name                     text        not null
  -- `content` for a page, `body` for a skill or agent. Fetched at ingest so a
  -- read is self-contained and does not depend on the source still existing.
, body                     text
  -- PROVENANCE — where the row came from. 'manifest' | 'generated' for skills
  -- and agents; the fetch origin for pages. Distinct from `source_type`, which
  -- is the ROUTE. Conflating the two is the trap 02b S11 names: both are
  -- plausibly "source" and nothing type-checks the difference.
, source                   text        not null default 'manifest'
, source_path              text
, version_range            text
  -- skill + agent
, focus                    text
  -- skill only
, tokens                   integer
, generated_at             timestamptz
  -- page only
, url                      text
, local_path               text
, description              text
, component                text
, source_type              library_source_type
, embedding                vector(768)
, fetched_at               timestamptz
  -- Shared vocabulary (see sensei.entity_scope / sensei.entity_origin).
  -- scope answers WHO MAY SEE IT and therefore whether it syncs; origin
  -- answers WHERE IT CAME FROM and therefore what a re-import may replace.
, scope                    entity_scope  not null default 'local'
, origin                   entity_origin not null default 'imported'
, modified_at              timestamptz not null default now()
  -- `kind` is IN the key. A library may legitimately ship a skill and a page
  -- both called "routing"; without the discriminator one would evict the other.
, unique (library_version_id, kind, name)
);

create index if not exists library_content_version_kind_idx
    on library_content(library_version_id, kind);

create index if not exists library_content_kind_idx
    on library_content(kind);

create index if not exists library_content_embedding_hnsw
    on library_content using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
 where embedding is not null;

comment on table library_content is
'Everything a library VERSION provides — skills, agents and documentation pages
in one table, discriminated by kind (R12).

Replaces library_skills, library_agents and library_pages, which shared nine
columns and differed in a handful. A fifth kind of content now costs a row
rather than a fourth table plus a fourth arm in every UNION.

Keyed to library_versions, not libraries: a library''s skills and docs can
differ between releases, and content hung off the identity could not say which
release it described.

Kind-specific columns are nullable BY DESIGN. Each belongs to exactly one kind
— focus/version_range to skills and agents, tokens/generated_at to skills,
url/local_path/description/component/source_type/embedding/fetched_at to pages
— so a null means "not applicable to this kind", never "unknown".';

comment on column library_content.id
     is 'Surrogate primary key (UUID).';
comment on column library_content.library_version_id
     is 'The library VERSION this content describes. Cascade-deletes with it.';
comment on column library_content.kind
     is 'skill | agent | page. Part of the unique key, because a library may ship a skill and a page under the same name.';
comment on column library_content.name
     is 'Skill/agent name, or a page title. Unique within (version, kind).';
comment on column library_content.body
     is 'The markdown or page content, fetched at ingest so a read is self-contained and survives the source being moved or deleted.';
comment on column library_content.source
     is 'PROVENANCE: manifest (declared in sensei.library.json) | generated (auto-generated by skill-gen) | the fetch origin for a page. Distinct from source_type, which is the ROUTE, not the origin.';
comment on column library_content.source_path
     is 'Manifest-relative path the body was read from. Null when generated.';
comment on column library_content.version_range
     is 'Semver range from the manifest that this content applies to. Free text; not enforced-matched.';
comment on column library_content.focus
     is 'Short topic key — the selector for get_library_skill ("styling"), or what an agent reviews. Skills and agents only.';
comment on column library_content.tokens
     is 'Approximate token size. Skills only; null until skill-gen runs.';
comment on column library_content.generated_at
     is 'When auto-generated. Skills only; null for manifest-declared content.';
comment on column library_content.url
     is 'Remote URL a page was fetched from (http or llms.txt route). Pages only.';
comment on column library_content.local_path
     is 'Filesystem path a page was read from (local route). Pages only.';
comment on column library_content.description
     is 'Short summary of a documentation page. Pages only.';
comment on column library_content.component
     is 'Sub-topic within the library ("routing", "middleware"). Pages only.';
comment on column library_content.source_type
     is 'The fetch ROUTE: llms.txt | http | local. Pages only. See `source` for the origin — the two are different questions.';
comment on column library_content.embedding
     is '768-dim vector for semantic search over documentation. Pages only.';
comment on column library_content.fetched_at
     is 'When this page was last fetched. Pages only; compare against library_versions.fetched_at to spot staleness.';
comment on column library_content.scope
     is 'Who may see it, and therefore whether it syncs. Shared vocabulary with sensei.entity_scope.';
comment on column library_content.origin
     is 'Where it came from, and therefore what a re-import may safely replace. Shared vocabulary with sensei.entity_origin.';
comment on column library_content.modified_at
     is 'Timestamp of the last modification to this row.';
