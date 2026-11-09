set search_path to relay, extensions;

-- The machine-local daemon user (local identity, user ruling 2026-11-09;
-- Q19 superseded): the stable id behind register and the person routes.
-- Minted once, never changes, never re-keyed — relay rows are keyed by it
-- on every machine, signed in or not, and the dōjō principal (when one
-- exists) never affects relay resolution. Sync maps local→principal at
-- the boundary; this row never leaves the machine (like `leases`).
--
-- SINGLE-ROW pattern (the `collective_preferences` precedent): `singleton`
-- is a boolean PK fixed to true by a CHECK — the PK's uniqueness plus the
-- CHECK make a second row impossible, and the mint keys on
-- `ON CONFLICT (singleton) DO NOTHING`.
create table if not exists relay.daemon_user (
  singleton            boolean           primary key default true  check (singleton),
  user_id              uuid              not null,
  minted_at            timestamptz       not null default current_timestamp,
  constraint daemon_user_id_v7 check (substring(user_id::text from 15 for 1) = '7')
);

comment on table relay.daemon_user is 'Single-row (singleton boolean PK) machine-local daemon user. Minted once on first relay use; register and the person routes resolve it always — no sign-in gate, no principal precedence (Q19 superseded 2026-11-09). Never synced: the dōjō boundary maps it to the linked principal.';
comment on column relay.daemon_user.singleton is 'Fixed true (CHECK) — enforces exactly one row; the mint keys ON CONFLICT (singleton) DO NOTHING.';
comment on column relay.daemon_user.user_id is 'The stable machine-local id. v7, like every relay id. Never changes, never re-keyed.';
comment on column relay.daemon_user.minted_at is 'When this machine minted its relay user.';

alter table relay.daemon_user enable row level security;
