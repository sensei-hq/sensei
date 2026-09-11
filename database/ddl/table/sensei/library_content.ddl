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
--   package_name           any kind, but in practice pages: WHICH published
--                          package this documents. NULL = library-level.
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
  -- WHICH PACKAGE this documents, when it documents one.
  --
  -- Skills and agents are library-level: rokkit's "styling" skill is about
  -- rokkit. DOCS are not. `rokkit` publishes `@rokkit/ui`, `@rokkit/core`,
  -- `@rokkit/actions` …, and the `List` page documents `@rokkit/ui`
  -- specifically. Without this column a reference to `@rokkit/ui` resolves to
  -- the library and then to ALL of its pages, and picking the right one falls
  -- back to matching `component` against a symbol name — a guess (R4).
  --
  -- NULL means library-level, which is a real and common state: an overview,
  -- a getting-started guide, an architecture page. NULL is "applies to the
  -- whole library", never "we don't know".
  --
  -- TEXT, and deliberately NOT a foreign key to `library_packages`. A page can
  -- name a package that has not been grouped yet, and an FK would either
  -- reject the page or force a phantom `library_packages` row — the
  -- get-or-create failure R13 forbids one table over. Join opportunistically;
  -- a miss is an honest "we hold no grouping for this package".
  --
  -- INVARIANT, not enforceable by a single FK: the named package must belong
  -- to the SAME library as this row's version. `library_packages.library_id`
  -- and `library_versions.library_id` must agree. A page claiming a package of
  -- a different library is a manifest conflict and gets REPORTED (02b §4),
  -- never silently resolved.
, package_name             text
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
  -- `kind` is IN the key: a library may legitimately ship a skill and a page
  -- both called "routing", and without the discriminator one would evict the
  -- other.
  --
  -- `package_name` is in it for the same reason one level down. `rokkit`
  -- publishes both `@rokkit/ui` and `@rokkit/chart`, and each can document a
  -- component called `List`. Keyed without the package, the second `List`
  -- page upserts over the first and one of them is simply lost.
  --
  -- Declared as a standalone index below, NOT as an inline constraint: the
  -- NULLS NOT DISTINCT clause is load-bearing and an inline
  -- `unique nulls not distinct (...)` is silently reduced to a plain unique
  -- by the schema tool, which is exactly the kind of quiet downgrade that
  -- looks applied and is not.
);

-- THE IDENTITY OF A PIECE OF CONTENT.
--
-- NULLS NOT DISTINCT is the load-bearing part. Postgres treats NULLs as
-- DISTINCT in a unique key by default, so the library-level rows — every
-- overview and guide, which are precisely the ones with no package — would not
-- be constrained at all, and re-ingesting a manifest would insert another
-- "Getting started" on every run instead of updating the one that exists.
create unique index if not exists library_content_identity_uq
    on library_content (library_version_id, kind, package_name, name)
       nulls not distinct;

create index if not exists library_content_version_kind_idx
    on library_content(library_version_id, kind);

-- The G1 lookup: a node references `@rokkit/ui`, and this is what turns that
-- into the pages documenting that package rather than the whole library's.
create index if not exists library_content_package_idx
    on library_content(package_name, kind)
 where package_name is not null;

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
— so a null means "not applicable to this kind", never "unknown".

package_name is the exception that is NOT kind-specific: it records WHICH of a
library''s published packages a row documents. Skills and agents are
library-level; docs are often package-level (rokkit''s List page documents
@rokkit/ui). NULL means library-level, which is a real state, not a gap.';

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
comment on column library_content.package_name
     is 'Which published package this row documents, as a dependency file spells it (@rokkit/ui) — matching library_packages.package_name, so resolution is an equality join. NULL means library-level: an overview or guide that is about the library as a whole, which is a real state and not a missing value. Plain TEXT, not an FK: a page can name a package not yet grouped, and an FK would force a phantom library_packages row. The named package MUST belong to the same library as this row''s version; a cross-library claim is a conflict to report, not to resolve.';
comment on column library_content.component
     is 'Sub-topic within a page set ("routing", "List"). Orthogonal to package_name: package_name says WHICH PACKAGE, component says WHICH TOPIC within it. Pages only.';
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
