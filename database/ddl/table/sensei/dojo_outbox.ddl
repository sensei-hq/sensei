set search_path to sensei, extensions;

-- The daemon's DURABLE upstream-contribution outbox (C6).
--
-- When an approved memory-share batch is contributed to one or many Dōjōs, each
-- (destination membership × source memory) produces one artifact. This table is
-- the send-ledger the daemon consults so a federation drop replays WITHOUT
-- double-sending: the unique (membership_id, signature) key means a given
-- artifact signature can be published to a given Dōjō at most once, and a retry
-- after a reconnect finds the prior row and skips (or re-attempts a queued one)
-- rather than publishing a duplicate.
--
-- Why a dedicated table and not extra columns on memory_share_batches: a batch
-- fans out to MULTIPLE destinations × MULTIPLE memories, each with its own
-- artifact signature and send state — there is no per-(destination, artifact)
-- slot on the batch row. This mirrors the sensei.federated_memories ledger that
-- the rules-federation push path uses for the same idempotency guarantee.
--
-- The dedup key is the dojo-protocol artifact `signature` (a content hash over
-- kind/title/body/payload of the CHECKED, publish-safe text), so the ledger is
-- keyed by exactly what leaves the machine.
create table if not exists dojo_outbox (
  id              uuid        primary key default gen_random_uuid()
, membership_id   uuid        not null
      references sensei.dojo_memberships(id) on delete cascade   -- destination Dōjō
, batch_id        uuid
      references sensei.memory_share_batches(id) on delete set null  -- source batch (audit)
, memory_id       uuid
      references sensei.memories(id) on delete set null              -- source memory (audit)
, signature       text        not null    -- dojo-protocol artifact signature (dedup key)
, state           text        not null default 'pending'
      check (state in ('pending', 'sent', 'held', 'queued', 'error'))
, sent_seq        bigint                  -- federation cursor seq assigned by the Dōjō on send
, remote_id       text                    -- dojo.artifacts.id assigned by the Dōjō on send
, last_attempt_at timestamptz
, created_at      timestamptz not null default now()
, updated_at      timestamptz not null default now()
, unique (membership_id, signature)   -- an artifact publishes to one Dōjō at most once
);

create index if not exists dojo_outbox_state_idx on dojo_outbox(state);
create index if not exists dojo_outbox_batch_idx on dojo_outbox(batch_id);
-- Covering index for the memory_id FK (nullable) — avoids a seq-scan when the
-- source memory is deleted (on delete set null). membership_id is already covered
-- by the unique (membership_id, signature) key.
create index if not exists dojo_outbox_memory_idx on dojo_outbox(memory_id) where memory_id is not null;

comment on table dojo_outbox is
'Durable upstream-contribution ledger (C6). One row per (destination membership,
artifact signature): a batch approved for sharing fans out to every routed Dōjō
and every member memory, each producing an artifact whose send state is tracked
here. The unique (membership_id, signature) key makes replay after a federation
drop idempotent — a retry never double-publishes. Mirrors the
sensei.federated_memories ledger used by the rules-federation push path.
- pending: planned, not yet attempted
- sent:    published; sent_seq / remote_id carry the Dōjō-assigned identity
- held:    confidentiality gate refused it (residual identifier risk) — not sent
- queued:  a transient (network / 5xx) failure — awaits replay
- error:   a permanent (4xx / undecodable) failure — logged, not auto-retried';

comment on column dojo_outbox.membership_id
     is 'Destination Dōjō (sensei.dojo_memberships.id). Cascade-deletes with the membership.';
comment on column dojo_outbox.signature
     is 'dojo-protocol artifact signature (content hash over the CHECKED publish-safe kind/title/body/payload). The dedup key: unique per membership.';
comment on column dojo_outbox.state
     is 'pending | sent | held | queued | error. `held` is the confidentiality outcome (dereference could not guarantee safety); `queued` is a retryable transport failure.';
comment on column dojo_outbox.sent_seq
     is 'The federation cursor seq the Dōjō assigned on publish. NULL until sent.';
comment on column dojo_outbox.remote_id
     is 'The dojo.artifacts.id the Dōjō assigned on publish. NULL until sent.';
