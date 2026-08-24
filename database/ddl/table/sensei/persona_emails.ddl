set search_path to sensei, extensions;

-- The git author emails a persona commits under.
--
-- This is what turns `repository_metrics.identity` (a raw git email) into a
-- readable identity. Without it, "my metrics" is fragmented across every address
-- the user has ever configured in a gitconfig.
--
-- SOURCE matters for trust. A `git` row was observed in commit history and is an
-- UNVERIFIED assertion — anyone can `git config user.email` to any address, so a
-- git row is fine locally (own machine, own data) but must never, by itself,
-- attribute work to a person in dōjō. A `claimed` row has been matched against a
-- provider-verified address, which is a real proof of mailbox control.
--
-- Soft delete, not hard: a removed address still explains historical rows, and
-- the auth flow needs to distinguish "never linked" from "linked and later
-- removed" (an email disappearing from a provider disables a membership rather
-- than erasing the past).
create table if not exists persona_emails (
  persona_id  uuid        not null references sensei.personas(id) on delete cascade
, email       text        not null
, source      text        not null default 'git'
      check (source in ('git', 'claimed'))
, linked_at   timestamptz not null default now()
, removed_at  timestamptz
, primary key (persona_id, email)
);

-- One live persona per address, case-insensitively.
--
-- `lower(email)` rather than a citext column: git emails are case-insensitive in
-- practice but stored as authored, and a functional index gives the same
-- guarantee without adding an extension the dōjō deployment (which ships no
-- extensions) would then have to carry.
--
-- Partial on `removed_at IS NULL` so an address can be re-linked after removal
-- instead of being permanently burned by its own history.
create unique index if not exists persona_emails_live_unique
    on persona_emails (lower(email)) where removed_at is null;

create index if not exists persona_emails_persona_idx
    on persona_emails (persona_id) where removed_at is null;

comment on table persona_emails is
'Git author emails belonging to a persona — the lookup that resolves
repository_metrics.identity to a readable working identity.

source = git: observed in commit history, UNVERIFIED (any author can write any
address). Usable locally; never sufficient to attribute work to a person in dōjō.
source = claimed: matched against a provider-verified address, so mailbox control
is proven.';

comment on column persona_emails.removed_at
     is 'Soft delete. A removed address still explains the rows it produced, and the auth flow needs "linked then removed" to be distinguishable from "never linked".';
