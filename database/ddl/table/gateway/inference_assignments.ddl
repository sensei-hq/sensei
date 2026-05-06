set search_path to gateway, extensions, sensei;

create table if not exists inference_assignments (
  id                       uuid            primary key default gen_random_uuid()
, role                     inference_role  not null
, chain_id                 uuid            not null references gateway.fallback_chains(id) on delete cascade
, is_default_fallback      boolean         not null default false
, modified_at              timestamptz     not null default now()
, unique(role, chain_id)
);

create index if not exists inference_assignments_role_idx
    on inference_assignments(role);

comment on table inference_assignments is
'Maps sensei inference roles to gateway fallback chains.
Each role (inference, consolidation, embedding, voice) is served by one or more
fallback chains. The default_fallback role is a catch-all chain used when a
role-specific chain is exhausted or missing.

- role: which reasoning purpose this chain serves
- chain_id: FK to fallback_chains — the ordered model sequence
- is_default_fallback: true on exactly one row per role — marks which chain
  to use as the last-resort fallback before giving up

Setup wizard (step 9) writes these rows. Users can later reconfigure from
Settings → Inference.';

comment on column inference_assignments.id
     is 'Surrogate primary key (UUID).';
comment on column inference_assignments.role
     is 'Inference role: inference (insights/recommendations), consolidation (memory merge/conflict), embedding (index/retrieval), voice (observatory speech), default_fallback (catch-all).';
comment on column inference_assignments.chain_id
     is 'Foreign key to fallback_chains — the ordered model sequence that serves this role.';
comment on column inference_assignments.is_default_fallback
     is 'When true, this chain is the last-resort fallback for the role. Exactly one per role.';
comment on column inference_assignments.modified_at
     is 'Timestamp of the last modification to this row.';
