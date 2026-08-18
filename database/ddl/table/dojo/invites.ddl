set search_path to dojo, extensions;

create table if not exists dojo.invites (
  id           uuid                  primary key default gen_random_uuid()
, tenant_id    uuid                  not null references dojo.tenants(id)
, email        text                  not null
, role         dojo.member_role      not null default 'contributor'
, kind         dojo.membership_kind  not null
, token        text                  not null
, invited_by   uuid                  not null
, expires_at   timestamptz           not null
, accepted_at  timestamptz
, created_at   timestamptz           not null default now()
, constraint invites_token_unique unique (token)
);

create index if not exists invites_tenant_idx on dojo.invites(tenant_id);
create index if not exists invites_email_idx on dojo.invites(lower(email));

comment on table dojo.invites is
'Magic-link membership invites (F3b). An admin issues one per (email, role,
kind) for their tenant; the invitee accepts it once, authenticated as that
email, and gets a membership at the invited role. The `token` is an unguessable
single-use bearer (crypto.randomUUID) carried in the accept link — but it is
NEVER sufficient alone: the accept path also requires the authenticated caller''s
email to equal `email` (Supabase magic-link proves ownership), so a leaked link
cannot be redeemed by anyone else. `accepted_at` makes it single-use; `expires_at`
bounds its life. Written/read only via the service_role (`/v1` routes with
app-level authz: admin floor to issue, email-match to accept) — no client-direct
access.';

comment on column dojo.invites.email
     is 'The invited email. The accept path requires the authenticated caller''s email to match (case-insensitive) — the real authorization gate, not the token.';
comment on column dojo.invites.token
     is 'Unguessable single-use bearer (crypto.randomUUID) in the accept link. Necessary but not sufficient — email-match + not-expired + not-accepted all gate the accept.';
comment on column dojo.invites.expires_at
     is 'Hard expiry; an accept after this is rejected (never creates a membership).';
comment on column dojo.invites.accepted_at
     is 'Set once on accept → single-use; a second accept is rejected.';

-- Locked down: only the service_role (the /v1 routes) touches invites. RLS on with an
-- explicit deny-all-to-clients policy (no `authenticated`/`anon` read or write) → the token
-- is never exposed to a browser query, only redeemed through the accept route. service_role
-- bypasses RLS. The explicit policy also clears the rls_enabled_no_policy advisor.
alter table dojo.invites enable row level security;
drop policy if exists invites_service_only on dojo.invites;
create policy invites_service_only on dojo.invites
    for all to authenticated, anon
    using (false) with check (false);
