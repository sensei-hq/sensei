set search_path to dojo, extensions;

create table if not exists dojo.memberships (
  id                   uuid                  primary key default gen_random_uuid()
, tenant_id            uuid                  not null references dojo.tenants(id)
, user_id              uuid                  not null
, role                 dojo.member_role      not null default 'contributor'
, kind                 dojo.membership_kind  not null
, org_slugs            text[]                not null default '{}'
, authenticated_via    dojo.auth_method      not null
, attribution_default  dojo.attribution_mode not null default 'named'
, sync_status          dojo.sync_status      not null default 'authenticating'
, device_key           text
, device_token_hash    text
, last_heartbeat_at    timestamptz
, disabled_at          timestamptz
, created_at           timestamptz           not null default now()
, updated_at           timestamptz           not null default now()
, constraint memberships_tenant_user_unique unique (tenant_id, user_id)
);

create index if not exists memberships_tenant_idx on dojo.memberships(tenant_id);
create index if not exists memberships_user_idx on dojo.memberships(user_id);
create index if not exists memberships_device_token_hash_idx on dojo.memberships(device_token_hash) where device_token_hash is not null;

comment on table dojo.memberships is
'A user''s participation in one Dōjō. A developer belongs to zero or many.
sensei.projects.dojo_id points at a row here (the binding that routes a
project''s findings). Rule: client kind takes precedence — a project bound to a
client membership routes to the client Dōjō first and the client policy
governs. The global collective is a membership with kind=community against the
global-dojo tenant.';

comment on column dojo.memberships.user_id
     is 'The member (Supabase auth subject). See dojo.identities. Not a local FK — identity is owned by Supabase.';
comment on column dojo.memberships.role
     is 'contributor | maintainer | lead | admin — usually git-derived (see dojo.roles), admin-overridable.';
comment on column dojo.memberships.kind
     is 'employer | client | community | personal — drives routing precedence and attribution.';
comment on column dojo.memberships.org_slugs
     is 'Git-remote owner slugs this membership covers (e.g. {sensei-hq,acme}), lowercased. A project whose repo owner is in this set is a candidate to auto-bind here (confirm-inferred in the app About panel). Set once at first-join/setup and overridable; feeds infer_binding with kind-precedence (client > employer > community > personal).';
comment on column dojo.memberships.authenticated_via
     is 'How this membership was paired: sso, github_oauth, or device_code.';
comment on column dojo.memberships.attribution_default
     is 'Default credit for contributions from this membership: named or anonymous. Source-dereference is a separate always-on invariant (client work is anonymous credit + stripped).';
comment on column dojo.memberships.sync_status
     is 'Last-known connection health (healthy/stale/error/authenticating), mirrored from the daemon for the connections pane.';
comment on column dojo.memberships.device_key
     is 'Public device key for verifying signed federation payloads, captured at membership creation.';
comment on column dojo.memberships.device_token_hash
     is 'sha256 of the daemon''s bearer device token (relay auth plane A, beta): the
relay routes authenticate a machine caller by hashing the presented Bearer and
matching this per (tenant, membership). Set by the seed / pairing; rotate via
re-pairing. The stronger signed-payload path (device_key above, plane B) is a
security-hardening follow-up before real/prod tokens.';
