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
create table if not exists personas (
  id            uuid        primary key default gen_random_uuid()
, label         text        not null
  -- FALSE for a contributor who is NOT the local user. `contributor@example.com`
  -- above may well be someone else; folding it into the user's own numbers would
  -- be a fabricated attribution, so an unrecognised email gets its own persona
  -- with is_self = false rather than a guess.
, is_self       boolean     not null default true
  -- The dōjō login this persona pushes under. Plain uuid, no FK: the referent
  -- lives in another database, exactly as `sensei.repositories.dojo_id` does.
  -- NULL until the user links this persona (Phase 6).
, principal_id  uuid
, created_at    timestamptz not null default now()
, modified_at   timestamptz not null default now()
);

create unique index if not exists personas_label_unique on personas (lower(label));

-- At most ONE persona per dōjō login. This is the constraint that makes the
-- separation enforceable: Supabase auto-links identities sharing a verified
-- email and cannot be told not to, so two personas CAN end up pointing at one
-- merged account. With this index that fails loudly at the point of linking,
-- instead of silently filing sensei-hq work under the personal identity.
create unique index if not exists personas_principal_unique
    on personas (principal_id) where principal_id is not null;

comment on table personas is
'A working identity kept separate from the user''s others — business vs personal
vs employer. Local-only: personas group git author emails so metrics can be read
per identity, and each may link to at most one dōjō login.

Not a "people" table. One human may own several personas by choice, and a
persona may belong to someone else entirely (is_self = false).';

comment on column personas.label
     is 'How the user names this identity: sensei-hq, personal, example-corp. Unique case-insensitively.';
comment on column personas.is_self
     is 'FALSE when this persona is another contributor, not the local user — so their commits are never counted as "mine".';
comment on column personas.principal_id
     is 'The dōjō login this persona pushes under (dojo.principals.id). Plain uuid — the referent is in another database. NULL until linked.';
