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
, access       text                  not null default 'member'
      check (access in ('member', 'viewer'))
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

comment on column dojo.invites.access
     is 'member: accepting makes a membership at `role`. viewer (#307 D8): accepting makes the person a dōjō viewer — not a member — whom an admin then grants the view of selected workspaces.';
comment on column dojo.invites.email
     is 'The invited email. The accept path requires the authenticated caller''s email to match (case-insensitive) — the real authorization gate, not the token.';
comment on column dojo.invites.token
     is 'Unguessable single-use bearer (crypto.randomUUID) in the accept link. Necessary but not sufficient — email-match + not-expired + not-accepted all gate the accept.';
comment on column dojo.invites.expires_at
     is 'Hard expiry; an accept after this is rejected (never creates a membership).';
comment on column dojo.invites.accepted_at
     is 'Set once on accept → single-use; a second accept is rejected.';

-- RLS: a tenant admin issues and reads the tenant's invites. The token is never
-- exposed to a session — the read grant leaves the column out — and is redeemed only
-- through the accept route (service_role). The policies call the access predicates
-- and so live in policies/dojo/invites.sql (see policies/README.md).
alter table dojo.invites enable row level security;
