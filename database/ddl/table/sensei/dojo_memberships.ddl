set search_path to sensei, extensions;

-- The daemon's LOCAL record of each Dōjō it is connected to.
--
-- Fork 1: dojo.* (memberships/tenants/artifacts/...) physically lives in the
-- Dōjō service's own Postgres, NOT in this `sensei` DB (see database/design.yaml
-- — the daemon's `default` scope EXCLUDES the `dojo` schema). So the daemon
-- keeps a small local mirror here to know where it is a member and how to reach
-- the service. It holds no credential: signing in is joining (#236 step 4e), so
-- each row is created by the sync from the signed-in persona's memberships and
-- its tenant calls present that persona's session. It is a SEPARATE table (not
-- a knowledge_sources kind) because a membership carries first-class routing
-- semantics — `kind` drives client-precedence routing — that do not belong
-- bolted onto the rules-federation table.
--
-- `id` is the SERVICE membership id (dojo.memberships.id), as the dōjō lists it —
-- NOT locally generated. That makes the one uuid mean
-- "this membership" everywhere: in the service DB, in this local mirror, and in
-- sensei.projects.dojo_id (which points here; no cross-DB FK — Fork 1).
create table if not exists dojo_memberships (
  id                   uuid        primary key   -- = dojo.memberships.id (service-assigned)
, registry_url         text        not null      -- the dōjō that issued the persona's session (dojo_client::settings::dojo_url)
, tenant_key           text        not null      -- <origin>/<org>[/<dojo>] discovery path of the tenant Dōjō
, dojo_url             text        not null      -- full membership URL (registry_url + tenant path)
, kind                 text        not null      -- employer | client | community | personal (drives routing precedence)
, org_slugs            text[]      not null default '{}'   -- git-remote owner slugs this membership covers (lowercased); mirrors dojo.memberships.org_slugs
, role                 text        not null default 'contributor'   -- contributor | maintainer | lead | admin
, authenticated_via    text        not null default 'device_code'   -- sso | github_oauth | device_code
, attribution_default  text        not null default 'named'         -- named | anonymous
, sync_status          text        not null default 'authenticating' -- healthy | stale | error | authenticating
, last_seq             bigint      not null default 0    -- downstream-artifact pull cursor (C7): the last dojo.artifacts seq mirrored into sensei.dojo_inbox
, last_heartbeat_at    timestamptz               -- intended: last successful federation heartbeat. NO WRITER EXISTS — see the column comment below
, enabled              boolean     not null default true
, created_at           timestamptz not null default now()
, updated_at           timestamptz not null default now()
);

create index if not exists dojo_memberships_kind_idx on dojo_memberships(kind);

comment on table dojo_memberships is
'Daemon-local mirror of the Dōjōs this install belongs to. Fork 1: the
authoritative dojo.memberships row lives in the Dōjō service DB; this row records
the registry URL, tenant_key, service membership id (as the PK) and routing
fields so the daemon can route and federate. sensei.projects.dojo_id points at
id. Created by the sync from the signed-in persona''s memberships (#236 step 4e);
holds no credential.';

comment on column dojo_memberships.id
     is 'The service membership id (dojo.memberships.id), as GET /v1/you/memberships lists it. sensei.projects.dojo_id references this value.';
comment on column dojo_memberships.kind
     is 'employer | client | community | personal — drives client-precedence routing (see dojo/routing.rs). client-bound work routes to the client Dōjō (credited anonymous); employer is excluded. All shared work is source-dereferenced regardless (always-on invariant).';
comment on column dojo_memberships.org_slugs
     is 'Git-remote owner slugs this membership covers (lowercased), mirrored from dojo.memberships.org_slugs. dojo/routing.rs::infer_binding matches a project''s repo owner against this set to SUGGEST a binding (confirm-inferred in the About panel) — it never auto-commits.';
comment on column dojo_memberships.sync_status
     is 'Intended: last-known connection health (healthy | stale | error | authenticating), derived on the daemon side from last_heartbeat_at. NOT IMPLEMENTED — the producer does not exist. Nothing writes last_heartbeat_at anywhere in the codebase, and the only sync_status setter (dojo::memberships::set_sync_status) is dead code with no callers, so every row is stuck at the ''authenticating'' literal written once when the sync joins it. Treat this column as a designed seam, not as data.';

comment on column dojo_memberships.last_heartbeat_at
     is 'Intended: the last successful federation heartbeat, driving the sync_status derivation. HAS NO WRITER. There is no heartbeat endpoint and no UPDATE against this column anywhere — daemon, dojo or app. The only real heartbeat in the system is activity.runs.heartbeat_at, which is run liveness and never touches memberships. Always NULL today.';
