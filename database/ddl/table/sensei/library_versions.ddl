set search_path to sensei, extensions;

-- S7b / 02b S10. ONE FETCHED VERSION of a library's documentation.
--
-- Library CONTENT — pages, skills, agents — hangs off a VERSION, not off the
-- library, so docs can be version-specific. Without this level, a project
-- pinned at 1.2 is served whatever the library's single row happens to hold,
-- which in practice is the LATEST docs: the higher-quality source of the wrong
-- answer (02b S9).
--
-- THE FETCH COLUMNS LIVE HERE, NOT ON `libraries`, and the distinction is
-- exactly the staleness problem. A website documents one version — normally
-- latest — so its `docs_url` goes stale for every older pin. GitHub carries a
-- URL per tag. So `base_url` / `docs_url` / `source_type` / `local_path` /
-- `page_count` are properties of a VERSION.
--
-- `homepage_url` deliberately stays on `libraries`: it is the PROJECT's home,
-- and a repository URL does not change per release.
--
-- `library_packages` is deliberately NOT versioned. The node -> package ->
-- library chain (R10.7g) starts from an fqn that carries no version, so a
-- version-keyed grouping could not be walked from a node at all.
create table if not exists library_versions (
  id               uuid        primary key default gen_random_uuid()
, library_id       uuid        not null references sensei.libraries(id) on delete cascade
  -- An ACTUAL version, or the literal 'unknown' when it has not been
  -- established. NEVER 'latest' — that is a TAG (see is_latest), not a
  -- version, and conflating them fabricates: at migration rokkit's 94 pages
  -- were keyed 'latest' while its real version, 1.4.1, sat in the
  -- package.json on disk. 'unknown' says what we know; 'latest' claimed
  -- something we had not established.
, version          text        not null
, resolved_version text
, source_type      sensei.library_source_type
, base_url         text
, docs_url         text
, local_path       text
, page_count       integer     not null default 0
  -- THE TAG. Which registered version is currently the latest, as a movable
  -- pointer rather than a magic value in `version`. At most one per library,
  -- enforced by a partial unique index — not merely intended.
, is_latest        boolean     not null default false
, fetched_at       timestamptz
, props            jsonb       not null default '{}'
, modified_at      timestamptz not null default now()
, unique (library_id, version)
);

create index if not exists library_versions_library_id_idx
    on library_versions(library_id);

create unique index if not exists library_versions_one_latest
    on library_versions(library_id) where is_latest;

comment on table library_versions is
'One fetched version of a library''s documentation (S7b). Pages, skills and
agents reference a VERSION so docs can be version-specific; a project pinned at
1.2 must not be served a website''s latest-only docs as though they matched.
The fetch columns (source_type, base_url, docs_url, local_path, page_count)
live here rather than on libraries because they are version-scoped — a docs URL
goes stale for older pins, while a repo homepage does not, which is why
libraries.homepage_url stays put.';

comment on column library_versions.version
     is 'An ACTUAL version, or ''unknown'' when it has not been established. NEVER ''latest'' — latest is a TAG (is_latest), not a version. Keying content on ''latest'' fabricates: at migration rokkit''s 94 pages were keyed that way while its real version, 1.4.1, was readable from package.json on disk.';
comment on column library_versions.is_latest
     is 'Whether this is the library''s current version — a movable TAG, at most one per library (partial unique index). Separate from `version` deliberately: "which version is this" and "is this the current one" are different questions, and answering the first with the second is how a stale row starts claiming to be current.';
comment on column library_versions.resolved_version
     is 'The concrete version a fetch resolved to when the request was for a moving target. NULL when unknown. This is what makes "is our latest still latest?" answerable — the question library_update_scheduler asks.';
comment on column library_versions.source_type
     is 'How THIS version''s pages were fetched: llms.txt | http | local.';
comment on column library_versions.base_url
     is 'Docs root for this version. Version-scoped: a website documents one version and its URL is wrong for older pins.';
comment on column library_versions.docs_url
     is 'Docs entry point for this version. See base_url.';
comment on column library_versions.local_path
     is 'On-disk location of this version''s docs, when source_type is local.';
comment on column library_versions.page_count
     is 'Denormalised count of this version''s library_content rows of kind ''page'' — per version, not per library.';
comment on column library_versions.fetched_at
     is 'When this version''s content was last fetched. Migrated from libraries.indexed_at.';
