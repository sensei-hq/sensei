set search_path to gateway, extensions, sensei;
create table if not exists fallback_chains (
  id                       uuid               primary key default gen_random_uuid()
, name                     text               not null unique
, capability               model_capability   not null
, description              text
, role                     inference_role
, max_fallback_attempts    integer            not null default 3
, is_active                boolean            not null default true
, sequence                 integer            not null default 0
, modified_at              timestamptz        not null default now()
);

create unique index if not exists fallback_chains_role_uidx
    on fallback_chains(role) where role is not null;

comment on table fallback_chains is
'Ordered model fallback sequences per capability.
When a model fails (timeout, rate limit, error), the gateway tries the next in sequence.
- capability: which model_capability this chain handles (chat, reasoning, embed, etc.)
- role: which sensei inference role this chain serves (nullable — utility chains
  like consensus-* stay null). Enforced unique when set, so a role points at
  exactly one chain and vice versa.
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
comment on column fallback_chains.role
     is 'Sensei inference role this chain serves. Null for utility chains (e.g. consensus-*) that the gateway invokes by name for internal orchestration. Unique when set — one chain per role.';
comment on column fallback_chains.max_fallback_attempts
     is 'Maximum number of models to attempt before failing.';
comment on column fallback_chains.is_active
     is 'Whether this chain is available for use.';
comment on column fallback_chains.sequence
     is 'Display order — lower values shown first.';
comment on column fallback_chains.modified_at
     is 'Timestamp of the last modification to this row.';
