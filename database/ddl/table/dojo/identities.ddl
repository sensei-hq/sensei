set search_path to dojo, extensions;

-- How a human proves who they are. GLOBAL, not tenant-scoped.
--
-- This was `unique (tenant_id, provider, subject)`, which does not survive
-- contact with the onboarding flow it was written for. One GitHub sign-in
-- provisions a personal tenant plus one per organisation — say four — and would
-- therefore create FOUR identity rows carrying the same `(provider, subject)`,
-- differing only by tenant. "Which human is this" then has no single row to
-- point at, and `user_id` (the field meant to tie them together) has nothing to
-- derive itself from on a first sign-in.
--
-- The auth spec assumed the global form throughout: it matches on
-- `(provider=github_oauth, subject=<github_user_id>)` and states that a
-- `(provider, subject)` unique prevents duplicates under concurrent sign-in.
-- That constraint now exists.
--
-- An identity is tenant-INDEPENDENT: it says "this GitHub account is this
-- person". Which dōjōs that person belongs to is `dojo.memberships`, already
-- keyed `(tenant_id, user_id)` — which is what lets one login fan out to many
-- dōjōs without duplicating the identity.
create table if not exists dojo.identities (
  id            uuid             primary key default gen_random_uuid()
  -- The stable principal, never auth.users directly — see dojo.principals.
, principal_id  uuid             not null references dojo.principals(id) on delete cascade
, provider      dojo.auth_method not null
, subject       text             not null
, email         text
, display_name  text
, created_at    timestamptz      not null default now()
, last_login_at timestamptz
, constraint identities_provider_subject_unique unique (provider, subject)
);

create index if not exists identities_principal_idx on dojo.identities(principal_id);

comment on table dojo.identities is
'One row per (auth provider, subject) — a proof of who someone is, independent of
any tenant. A principal may hold several: GitHub today, Google tomorrow, the same
human.

Tenant membership lives in dojo.memberships. Keeping the two apart is what makes
"one login, many dōjōs" expressible; the previous tenant-scoped unique made it
impossible.';

comment on column dojo.identities.principal_id
     is 'The person this proof belongs to. FK to dojo.principals, never to auth.users — see that table for why.';
comment on column dojo.identities.provider
     is 'Which auth method produced this subject: sso, github_oauth, device_code, or azure_devops (an Azure DevOps account read through a Microsoft Entra connect, by its stable id, #308 R5b).';
comment on column dojo.identities.subject
     is 'The provider''s subject identifier (OIDC/SAML sub, GitHub user id, or device-code enrollment id).';

-- RLS: the access matrix's cells for this table. The policies call the access
-- predicates and so live in policies/dojo/identities.sql (see policies/README.md).
alter table dojo.identities enable row level security;
