set search_path to sensei, extensions;

-- A working identity — one row per identity the user keeps APART, not one row
-- per human.
--
-- The distinction matters and is the whole reason this table exists. Measured on
-- real data, six git author emails were writing user-scoped metrics:
--
--   me@jerrythomas.name             422 rows / 26 repos
--   hi@sensei-hq.com                108 / 2
--   owner@example.com       84 / 2
--   dev@sensei-hq.com                74 / 1
--   dev@example-corp.com    62 / 9
--   contributor@example.com        17 / 1
--
-- Those are not one person with six aliases, and they are not six people. They
-- are two or three deliberate working identities (a business one, a personal
-- one, an employer one) plus possibly a different human. Merging them all would
-- be as wrong as leaving them split: "my metrics" would silently include a
-- client's work, and in dōjō the sensei-hq identity would surface under the
-- personal account.
--
-- So: LOCALLY every persona is visible in one place, each row tagged with its
-- persona. REMOTELY they are separate logins. `principal_id` is the seam — at
-- most one login per persona — and the unique index is what makes the privacy
-- boundary structural rather than a convention.
--
-- LABEL IS THE GITHUB LOGIN, not a category of work. That distinction is
-- load-bearing: `jerrythomas` commits across EIGHT repository owners
-- (jerrythomas, example-corp-inc, example-health, example-alert, example-labs, sensei-hq, …), so a
-- label like "personal" would read as "personal repos" and be wrong on seven of
-- them. A persona answers WHO, never WHERE — and two personas can commit to the
-- same owner (both `jerrythomas` and `sensei-hq` touch sensei-hq repos, under
-- different addresses), which is precisely why the repo cannot imply the
-- identity.
create table if not exists personas (
  id            uuid        primary key default gen_random_uuid()
  -- DISPLAY name. Provisional until the persona is connected: before OAuth it
  -- can only be a guess derived from a git email or a repository owner, and
  -- guesses are wrong — `sensei-hq` was inferred from an email domain when the
  -- real GitHub login is `sensei-hq-org`.
, label         text        not null
  -- VERIFIED identity, set only by a completed OAuth sign-in. NULL means "we
  -- have not proven who this is", which is a different and more useful state
  -- than a plausible-looking label.
, github_login   text
  -- The stable GitHub id. A login can be RENAMED — the id cannot — so this, not
  -- the login, is what an identity is matched on across time.
, github_user_id bigint
, verified_at    timestamptz
  -- FALSE for a contributor who is NOT the local user. `contributor@example.com`
  -- above may well be someone else; folding it into the user's own numbers would
  -- be a fabricated attribution, so an unrecognised email gets its own persona
  -- with is_self = false rather than a guess.
, is_self       boolean     not null default true
  -- The dōjō login this persona pushes under. Plain uuid, no FK: the referent
  -- lives in another database, exactly as `sensei.repositories.tenant_id` does.
  -- NULL until the user links this persona (Phase 6).
, principal_id  uuid
  -- The KEYCHAIN SLOT this persona's dōjō session is stored under, as chosen by
  -- whoever started the sign-in (`/api/auth/signin?persona=X`).
  --
  -- Distinct from `label`, and that distinction is the whole reason the column
  -- exists. `label` is a DISPLAY name that a successful sign-in REWRITES to the
  -- verified GitHub login — a user who signs in as `default` ends up with a row
  -- labelled `sensei-hq-org`. An unattended task that looked up the session by
  -- `label` would read `refresh_token.sensei-hq-org`, find nothing, and skip the
  -- persona while reporting the cycle successful. Observed, not theorised.
, session_slot  text
, created_at    timestamptz not null default now()
, modified_at   timestamptz not null default now()
);

create unique index if not exists personas_label_unique on personas (lower(label));

-- One persona per GitHub account. Two personas resolving to the same account
-- would split one identity's history in half with no way to tell which is real.
create unique index if not exists personas_github_user_unique
    on personas (github_user_id) where github_user_id is not null;

-- At most ONE persona per dōjō login. This is the constraint that makes the
-- separation enforceable: Supabase auto-links identities sharing a verified
-- email and cannot be told not to, so two personas CAN end up pointing at one
-- merged account. With this index that fails loudly at the point of linking,
-- instead of silently filing sensei-hq work under the personal identity.
create unique index if not exists personas_principal_unique
    on personas (principal_id) where principal_id is not null;

-- One persona per Keychain slot. Two personas sharing a slot would mean one
-- stored session serving two identities, so an unattended sync would push one
-- person's metrics under the other's token.
create unique index if not exists personas_session_slot_unique
    on personas (session_slot) where session_slot is not null;

comment on table personas is
'A working identity kept separate from the user''s others — business vs personal
vs employer. Local-only: personas group git author emails so metrics can be read
per identity, and each may link to at most one dōjō login.

Not a "people" table. One human may own several personas by choice, and a
persona may belong to someone else entirely (is_self = false).';

comment on column personas.label
     is 'Display name, unique case-insensitively. PROVISIONAL until verified_at is set — before a sign-in it can only be inferred from a git email or repo owner, and such inferences are wrong (sensei-hq vs the real sensei-hq-org). NOT a category of repo — one persona spans many owners.';
comment on column personas.github_login
     is 'The GitHub login, proven by OAuth. NULL until the persona is connected — an unproven identity should look unproven rather than merely unlabelled.';
comment on column personas.github_user_id
     is 'GitHub''s stable numeric id. Matched on in preference to the login, which the user can rename.';
comment on column personas.verified_at
     is 'When OAuth last confirmed this identity. NULL = discovered from git only.';
comment on column personas.is_self
     is 'FALSE when this persona is another contributor, not the local user — so their commits are never counted as "mine".';
comment on column personas.principal_id
     is 'The dōjō login this persona pushes under (dojo.principals.id). Plain uuid — the referent is in another database. NULL until linked.';
comment on column personas.session_slot
     is 'The Keychain slot holding this persona''s dōjō session (session.rs::account_for formats refresh_token.<slot>). NOT the label: a successful sign-in rewrites label to the verified GitHub login, so signing in as "default" yields a row labelled "sensei-hq-org" whose session is still at refresh_token.default. Looking the session up by label finds nothing and silently skips the persona. NULL = never signed in from this machine.';
