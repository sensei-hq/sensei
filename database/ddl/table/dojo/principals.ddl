set search_path to dojo, extensions;

-- The stable identity every dōjō foreign key points at.
--
-- WHY THIS EXISTS RATHER THAN REFERENCING auth.users DIRECTLY.
--
-- Supabase automatically links identities that share a verified email address,
-- and this cannot be disabled — Auth is built on "all user emails are unique"
-- and the maintainers have declined a flag for it. So two accounts a user is
-- deliberately keeping apart CAN be merged into one `auth.users` row by nothing
-- more than an address appearing on both.
--
-- Supabase then offers no way back: `unlinkIdentity` deletes an identity rather
-- than splitting a user, and signing in again re-links while the email still
-- matches. There is no split-user operation at all.
--
-- Keying our data on `auth.users.id` would make that merge permanent for us too.
-- Keying on `principals.id` — with `auth_user_id` as a POINTER we can re-aim —
-- makes a split our own transaction: sign in fresh, point a second principal at
-- the new account, re-attribute the rows. The history follows because it never
-- referenced the login in the first place.
create table if not exists principals (
  id            uuid        primary key default gen_random_uuid()
  -- A POINTER, not the identity. Nullable so a principal can exist before (or
  -- after) it has a login — a re-attribution transiently has neither side
  -- attached, and an invited-but-unregistered member has no account yet.
, auth_user_id  uuid        unique
, display_name  text
, created_at    timestamptz not null default now()
, updated_at    timestamptz not null default now()
);

comment on table principals is
'The stable identity dōjō foreign keys reference. `auth_user_id` points at the
Supabase login and can be re-pointed; nothing else should reference auth.users
directly.

This indirection is what makes an accidentally-merged account recoverable.
Supabase auto-links identities sharing a verified email, cannot be told not to,
and provides no operation to split a user afterwards — so the only way to undo
one without losing history is to own the mapping ourselves.';

comment on column principals.auth_user_id
     is 'The Supabase auth.users row this principal currently signs in as. Re-pointable by design; UNIQUE so two principals can never claim one login.';
