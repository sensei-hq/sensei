set search_path to dojo, extensions;

-- A tenant's identity on ONE source forge: how an organization is known to
-- GitHub, GitLab, Bitbucket or Azure DevOps (it may be known to several), and how
-- a PERSONAL tenant is known — as its owner's own account (#301), so that
-- register_repository maps `{login}/…` repositories the same way it maps an org's.
--
-- Why this is a child table rather than columns on `tenants`: an org with both
-- a GitHub and an Azure presence is one dōjō, one subscription, one governance
-- set. Columns would force either two tenants (splitting the org) or a single
-- forge (the trap the `tenant_origin` enum used to be).
create table if not exists dojo.tenant_connections (
    id             uuid                primary key default gen_random_uuid()
  , tenant_id      uuid                not null references dojo.tenants(id) on delete cascade
  , provider       dojo.forge_provider not null
  , external_id    text                                    -- the forge's STABLE id; NULL until proven
  , external_slug  text                not null            -- the forge's name for the org
  , connected_by   uuid                not null            -- the user who linked it
  , verified_at    timestamptz                             -- when org control was last proven
  , token_secret_id  uuid
  , token_state      dojo.forge_token_state
  , token_expires_at timestamptz
  , members_read_at    timestamptz
  , members_read_error text
  , created_at     timestamptz         not null default now()
  , updated_at     timestamptz         not null default now()
);

-- One PROVEN forge org maps to at most one tenant, forever. Keyed on the stable
-- id and not the slug: a slug can be renamed and re-registered upstream, and
-- keying on it would let whoever claims the freed name inherit this tenant's
-- governance.
create unique index if not exists tenant_connections_external_id_uniq
    on dojo.tenant_connections (provider, external_id)
 where external_id is not null;

-- Two UNPROVEN connections must not race for the same slug either.
create unique index if not exists tenant_connections_unproven_slug_uniq
    on dojo.tenant_connections (provider, lower(external_slug))
 where external_id is null;

create index if not exists tenant_connections_tenant_idx
    on dojo.tenant_connections (tenant_id);

comment on table dojo.tenant_connections is
'A tenant''s identity on one source forge (spec dojo-auth-provisioning §II.2).
A tenant is an organization; this is how that organization is known to a given
forge, and it may be known to several. Linking a second forge is an authorized
human act — same slug is never evidence of same organization — performed by
someone who is authenticated on both sides AND already administers the tenant.';

comment on column dojo.tenant_connections.external_id is
'The forge''s stable identifier for the org (GitHub numeric id, Azure GUID).
NULLABLE by design: it is an enrichment that arrives when the API confirms the
org, and a migration or a rate-limited lookup may not have it yet. A connection
without one is UNVERIFIED and confers no entitlement — a tenant whose only
connection is unverified cannot be claimed, so it cannot hold billing and cannot
sync private data. That gate, not a NOT NULL, is what contains slug squatting.';

comment on column dojo.tenant_connections.external_slug is
'The forge''s name for the org. NOT NULL — it is how the org was found. Distinct
from dojo.tenants.slug, which is the tenant''s own name and may differ from
every forge''s spelling of it.';

comment on column dojo.tenant_connections.verified_at is
'When org control was last proven against the forge. NULL = unverified: the
connection exists but grants nothing. Re-proved on each provisioning pass, and
only a pass that positively read the forge may clear it — an API outage must
never be read as "the org is gone".';

comment on column dojo.tenant_connections.token_secret_id is
'The Supabase Vault secret (vault.secrets.id) holding this connection''s forge
grant (#308 R5b, D42: the Microsoft Entra refresh token of the Azure DevOps admin
who connected; access tokens are never stored). NULL when none. Never the token:
it is written, rotated, replaced, removed and read only through
dojo.set_forge_token / rotate_forge_token / remove_forge_token / forge_token
(service_role only).';

comment on column dojo.tenant_connections.token_state is
'ok, or needs_new_token once the forge rejected the stored grant (a refresh
answered invalid_grant: revoked, expired, or consent withdrawn) — reads stop until
an admin reconnects. NULL when no token is stored.';

comment on column dojo.tenant_connections.token_expires_at is
'When the stored grant expires, when the forge says (NULL when unknown — Entra does
not report a refresh token''s lifetime).';

comment on column dojo.tenant_connections.members_read_at is
'When the scheduled member read last read this connection (#308 R5b, D44) — the
run reads the longest-unread first. NULL until it has.';

comment on column dojo.tenant_connections.members_read_error is
'Why the last scheduled member read of this connection ended nobody (a failed,
partial or refused read), or NULL when it read completely. Never a token.';

-- RLS on, and no session grant: read only by provisioning (the system) and
-- dojo.register_repository, which returns a reason, never another tenant's id (#236).
alter table dojo.tenant_connections enable row level security;
