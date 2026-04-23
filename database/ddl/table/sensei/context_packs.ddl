set search_path to sensei, extensions;

create table if not exists context_packs (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, session_id               uuid
, task                     text        not null
, model_id                 text
, slices                   jsonb       not null default '[]'
, total_tokens             integer     not null default 0
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
);

create index if not exists context_packs_folder_id_idx
    on context_packs(folder_id, created_at desc);

create index if not exists context_packs_session_id_idx
    on context_packs(session_id)
 where session_id is not null;

comment on table context_packs is
'Assembled context packs persisted for token usage analytics.
- slices: jsonb array of file slices included in the pack
- total_tokens: total token count of the assembled pack';

comment on column context_packs.id
     is 'Surrogate primary key (UUID).';
comment on column context_packs.folder_id
     is 'Foreign key to folders — which folder this pack was assembled for.';
comment on column context_packs.session_id
     is 'Session that created this pack. Nullable.';
comment on column context_packs.task
     is 'Task or query string the context pack was assembled for.';
comment on column context_packs.model_id
     is 'Identifier of the model the context pack was assembled for.';
comment on column context_packs.slices
     is 'JSON array of file slices included in the assembled pack.';
comment on column context_packs.total_tokens
     is 'Total token count of all slices in the assembled pack.';
comment on column context_packs.created_at
     is 'Timestamp when the row was first created.';
comment on column context_packs.modified_at
     is 'Timestamp of the last modification to this row.';
