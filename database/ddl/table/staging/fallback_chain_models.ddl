set search_path to staging, extensions;

drop table if exists fallback_chain_models cascade;
create table fallback_chain_models (
  chain_name               text
, router_name              text
, model_full_name          text
, sequence_order           integer
, max_retries              integer     default 1
, is_active                boolean     default true
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, modified_by              text        not null default current_user
);
