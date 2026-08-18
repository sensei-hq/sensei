set search_path to sensei, extensions;

-- The daemon's LOCAL downstream inbox (C7) — approved Dōjō artifacts pulled back
-- for the user to review and Apply.
--
-- Fork 1: the authoritative distribution ledger is dojo.downstream_inbox in the
-- Dōjō service DB. The daemon pulls each membership's published artifacts (the
-- `dojo`-style seq cursor lives on sensei.dojo_memberships.last_seq) and mirrors
-- them here as `pending`, so the Upgrades screen has a local, offline-readable
-- inbox and mute/pin overrides are honoured locally without a round-trip.
--
-- This is the DOWNSTREAM twin of sensei.dojo_outbox (the C6 upstream send-ledger):
-- outbox tracks what we PUBLISH, inbox tracks what we RECEIVE. The dedup key is
-- the dojo-protocol artifact `signature` (unique per membership), so a re-pull can
-- never duplicate a row — mirrors the outbox's (membership_id, signature) key.
--
-- Landing (Apply):
--   principle | pattern → a sensei.memories row with origin='dojo' (applied_memory_id
--                         links back); state → 'applied'.
--   skill | agent | prompt | guard → NOT auto-installed (a consent-sensitive plugin
--                         / lint surface, deferred to a follow-up); the row stays
--                         `pending` with a `note`, and Apply is a no-op-with-reason.
-- mute hides an item from the default Upgrades list; pin floats it to the top and
-- lets it outrank ambiguous local alternatives. Neither lands anything.
create table if not exists dojo_inbox (
  id                 uuid        primary key default gen_random_uuid()
, membership_id      uuid        not null
      references sensei.dojo_memberships(id) on delete cascade   -- source Dōjō connection
, artifact_seq       bigint      not null    -- the service federation cursor seq of the source artifact
, artifact_signature text        not null    -- dojo-protocol artifact signature (dedup key)
, remote_id          text                    -- dojo.artifacts.id assigned by the service (provenance)
, kind               text        not null    -- principle | pattern | prompt | guard | skill | agent
, title              text        not null
, body               text        not null
, scope              jsonb       not null default '{}'   -- ArtifactScope {company,team,project,stack}
, attribution        jsonb       not null default '{}'   -- Attribution {mode,author,org,anonymous_id}
, state              text        not null default 'pending'
      check (state in ('pending', 'applied', 'muted', 'pinned'))
, note               text                    -- reason Apply was deferred / not landed (deferred kind, scope mismatch)
, applied_memory_id  uuid        references sensei.memories(id) on delete set null  -- the landed memory (principle/pattern)
, received_at        timestamptz not null default now()
, updated_at         timestamptz not null default now()
, unique (membership_id, artifact_signature)   -- a re-pull never duplicates an artifact
);

create index if not exists dojo_inbox_state_idx on dojo_inbox(state);
create index if not exists dojo_inbox_membership_idx on dojo_inbox(membership_id);
-- Covering index for the applied_memory_id FK (nullable) — avoids a seq-scan when
-- the landed memory is deleted (on delete set null).
create index if not exists dojo_inbox_applied_memory_idx on dojo_inbox(applied_memory_id) where applied_memory_id is not null;

comment on table dojo_inbox is
'Daemon-local downstream inbox (C7). One row per (source membership, pulled
artifact): the Dōjō service distributes approved artifacts to a membership; the
daemon pulls them (cursor = sensei.dojo_memberships.last_seq) and mirrors them
here as `pending` for the Upgrades screen. The DOWNSTREAM twin of
sensei.dojo_outbox. Apply lands a principle/pattern as a sensei.memories row
(origin=''dojo'', applied_memory_id links it); skill/agent/prompt/guard are held
pending (consent-sensitive install, deferred). mute/pin are local overrides that
never land anything. The unique (membership_id, artifact_signature) key makes a
re-pull idempotent.';

comment on column dojo_inbox.membership_id
     is 'Source Dōjō connection (sensei.dojo_memberships.id). Cascade-deletes with the membership.';
comment on column dojo_inbox.artifact_seq
     is 'The federation cursor seq of the source artifact (dojo.artifacts seq). The per-membership pull cursor itself lives on sensei.dojo_memberships.last_seq.';
comment on column dojo_inbox.artifact_signature
     is 'dojo-protocol artifact signature (content hash over kind/title/body/payload). The dedup key: unique per membership so a re-pull cannot duplicate.';
comment on column dojo_inbox.kind
     is 'principle | pattern | prompt | guard | skill | agent. Only principle/pattern auto-land on Apply (→ sensei.memories); the rest stay pending with a note.';
comment on column dojo_inbox.state
     is 'pending | applied | muted | pinned. applied = landed into sensei.memories (applied_memory_id set); muted = hidden from the default Upgrades list; pinned = floated to the top / outranks ambiguous local alternatives.';
comment on column dojo_inbox.note
     is 'Reason an Apply did not land: a deferred kind (skill/agent/prompt/guard) or a scope that does not apply to this install. Null otherwise.';
comment on column dojo_inbox.applied_memory_id
     is 'The sensei.memories row this artifact landed as (origin=dojo). Null until applied (or for kinds that do not land).';
