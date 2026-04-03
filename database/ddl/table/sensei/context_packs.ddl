set search_path to sensei, extensions;

create table if not exists context_packs (
  id           uuid        primary key default gen_random_uuid()
, repo_id      uuid        not null references sensei.repos(id) on delete cascade
, session_id   uuid        references sensei.sessions(id) on delete set null
, task         text        not null
, model_id     text
, slices       jsonb       not null default '[]'
, total_tokens integer     not null default 0
, created_at   timestamptz not null default now()
);

create index if not exists context_packs_repo_id_idx    on context_packs(repo_id, created_at desc);
create index if not exists context_packs_session_id_idx on context_packs(session_id)
  where session_id is not null;

comment on table context_packs is
'Assembled context packs persisted for token usage analytics.
- slices: jsonb array of file slices included in the pack
- total_tokens: total token count of the assembled pack';
