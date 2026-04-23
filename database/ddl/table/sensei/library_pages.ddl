set search_path to sensei, extensions;

create table if not exists library_pages (
  id                       uuid        primary key default gen_random_uuid()
, library_id               uuid        not null references sensei.libraries(id) on delete cascade
, title                    text        not null
, url                      text
, local_path               text
, description              text
, content                  text
, source_type              text        not null
                                       check (source_type in ('llms.txt', 'http', 'local'))
, component                text
, embedding                vector(768)
, fetched_at               timestamptz
, modified_at              timestamptz not null default now()
, created_at               timestamptz not null default now()
);

create index if not exists library_pages_library_id_idx
    on library_pages(library_id);

create index if not exists library_pages_embedding_hnsw
    on library_pages using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
 where embedding is not null;

comment on table library_pages is
'Documentation pages/sections for libraries.
Consolidates the former lib_doc_sections, shared_lib_sections, and lib_docs tables.
- One row per content page fetched from a library source (llms.txt, HTTP, or local)
- url: the remote URL if fetched over HTTP (absolute URL or path from llms.txt)
- local_path: filesystem path if sourced locally
- component: sub-topic within the library (e.g. "routing", "middleware")
- embedding: 768-dim vector for semantic search';

comment on column library_pages.id
     is 'Surrogate primary key (UUID).';
comment on column library_pages.library_id
     is 'Foreign key to libraries — which library this page belongs to.';
comment on column library_pages.title
     is 'Title of this documentation page/section.';
comment on column library_pages.url
     is 'Remote URL this page was fetched from, if sourced over HTTP or llms.txt.';
comment on column library_pages.local_path
     is 'Local filesystem path this page was read from, if sourced locally.';
comment on column library_pages.description
     is 'Short summary of this page content.';
comment on column library_pages.content
     is 'Full text content of this documentation page.';
comment on column library_pages.source_type
     is 'How this page was obtained: llms.txt, http, or local.';
comment on column library_pages.component
     is 'Optional sub-component or topic label within the library.';
comment on column library_pages.embedding
     is '768-dimensional vector embedding for semantic search.';
comment on column library_pages.fetched_at
     is 'Timestamp when this page content was last fetched or refreshed.';
comment on column library_pages.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column library_pages.created_at
     is 'Timestamp when this page was first indexed.';
