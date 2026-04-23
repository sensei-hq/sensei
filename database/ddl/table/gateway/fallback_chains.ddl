set search_path to gateway, extensions;

create table if not exists fallback_chains (
  id                       uuid               primary key default gen_random_uuid()
, name                     text               not null unique
, capability               model_capability   not null
, description              text
, max_fallback_attempts    integer            not null default 3
, is_active                boolean            not null default true
, sequence                 integer            not null default 0
, modified_at              timestamptz        not null default now()
, created_at               timestamptz        not null default now()
);

comment on table fallback_chains is
'Ordered model fallback sequences per capability.
When a model fails (timeout, rate limit, error), the gateway tries the next in sequence.
- capability: which model_capability this chain handles (chat, reasoning, embed, etc.)
- max_fallback_attempts: how many models to try before giving up
Seed data loaded via staging.import_fallback_chains().';

comment on column fallback_chains.id
     is 'Surrogate primary key (UUID).';
comment on column fallback_chains.name
     is 'Unique chain name (e.g. "reasoning", "embed", "classify", "consensus-proposer").';
comment on column fallback_chains.capability
     is 'Which capability this chain serves: chat, reasoning, embed, classify, summarize, vision, audio.';
comment on column fallback_chains.description
     is 'What this chain is for (e.g. "MOE proposer — strong reasoning models").';
comment on column fallback_chains.max_fallback_attempts
     is 'Maximum number of models to attempt before failing.';
comment on column fallback_chains.is_active
     is 'Whether this chain is available for use.';
comment on column fallback_chains.sequence
     is 'Display order — lower values shown first.';
comment on column fallback_chains.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column fallback_chains.created_at
     is 'Timestamp when this chain was first registered.';
