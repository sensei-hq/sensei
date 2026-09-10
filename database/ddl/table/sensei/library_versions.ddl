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
  -- The KEY, and it may be the literal 'latest'. Measured at migration: 130 of
  -- 130 pages belonged to libraries whose `version` was NULL, so keying on the
  -- observed column alone would have orphaned every page.
, version          text        not null
, resolved_version text
, source_type      sensei.library_source_type
, base_url         text
, docs_url         text
, local_path       text
, page_count       integer     not null default 0
, fetched_at       timestamptz
, props            jsonb       not null default '{}'
, modified_at      timestamptz not null default now()
, unique (library_id, version)
);

create index if not exists library_versions_library_id_idx
    on library_versions(library_id);

comment on table library_versions is
'One fetched version of a library''s documentation (S7b). Pages, skills and
agents reference a VERSION so docs can be version-specific; a project pinned at
1.2 must not be served a website''s latest-only docs as though they matched.
The fetch columns (source_type, base_url, docs_url, local_path, page_count)
live here rather than on libraries because they are version-scoped — a docs URL
goes stale for older pins, while a repo homepage does not, which is why
libraries.homepage_url stays put.';

comment on column library_versions.version
     is 'The KEY, and it may be the literal ''latest''. A library whose version was never observed is keyed ''latest'' — that is what we know, and it beats fabricating a version string. UNIQUE with library_id.';
comment on column library_versions.resolved_version
     is 'What ''latest'' actually resolved to at fetched_at. NULL when unknown — the honest state for a library whose version was never observed. Without this, "do we have latest?" is answerable but "is our latest still latest?" is not, and the second is what library_update_scheduler asks.';
comment on column library_versions.source_type
     is 'How THIS version''s pages were fetched: llms.txt | http | local.';
comment on column library_versions.base_url
     is 'Docs root for this version. Version-scoped: a website documents one version and its URL is wrong for older pins.';
comment on column library_versions.docs_url
     is 'Docs entry point for this version. See base_url.';
comment on column library_versions.local_path
     is 'On-disk location of this version''s docs, when source_type is local.';
comment on column library_versions.page_count
     is 'Denormalised count of this version''s library_pages — per version, not per library.';
comment on column library_versions.fetched_at
     is 'When this version''s content was last fetched. Migrated from libraries.indexed_at.';
