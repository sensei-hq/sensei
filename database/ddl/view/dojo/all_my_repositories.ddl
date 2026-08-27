set search_path to dojo, extensions;

-- "Which repositories are mine, and whose dōjō does each belong to?"
--
-- WHY THIS IS THE RIGHT GRAIN. The daemon and the console are both USER-plane
-- clients: a person belongs to several tenants, each tenant owns repositories,
-- and every repository belongs to exactly one tenant. So the useful question is
-- never "what does tenant X hold" — that would make the caller ask N times and
-- would require it to already know which tenant a repository belongs to, which
-- is the very thing it is asking. One read, with the owning tenant carried on
-- each row, answers it.
--
-- One row per (repository, member): a tenant's repository appears once for each
-- of its members, so `where principal_id = …` is the whole filter.
--
-- Backs BOTH `GET /v1/you/sync/plan` and the console's repository list, so the
-- two can never disagree about what a user has — which is the same reason the
-- daemon asks for a plan rather than caching one.
--
-- `sync_enabled` is constant TRUE in phase 1 because nothing gates yet: there is
-- no claim, no billing and no seat, so every registered repository syncs. It
-- exists now so the shape does not change when the phase-2 gate arrives (§IV.3)
-- and starts computing it — the same argument as the plan endpoint always
-- carrying `denied: []`. It is the genuine phase-1 answer, not a placeholder.
--
-- SECURITY: `security_invoker = on` — runs as the calling role, not the owner
-- (Supabase lint security_definer_view). Read server-side as service_role; the
-- caller supplies the principal filter. Not granted to any client role, so a
-- user's repository list is never exposed via PostgREST.
create or replace view dojo.all_my_repositories
with (security_invoker = on)
as
select r.id            as repository_id
     , r.repo_key
     , r.name
     , r.remote_url
     , r.provider
     , r.visibility                       -- the forge's answer; phase-2 gate input
     , t.id            as tenant_id
     , t.key           as tenant          -- the discovery path <origin>/<slug>
     , t.slug          as owning_org
     , t.origin
     , m.user_id       as principal_id    -- whose row this is; the filter column
     , m.role
     , true            as sync_enabled
       -- Why a denial happened, when one does. NULL throughout phase 1 because
       -- nothing denies yet; in phase 2 it carries `unclaimed` / `not_subscribed`
       -- / `no_seat` / `subscription_expired` (§IV.3). Present now so the plan
       -- endpoint reads the same columns before and after the gate arrives.
     , null::text      as denied_reason
  from dojo.repositories r
  join dojo.tenants t     on t.id = r.tenant_id
  join dojo.memberships m on m.tenant_id = r.tenant_id
                        and m.disabled_at is null;

comment on view dojo.all_my_repositories is
'Every repository a user can reach, with its owning tenant on the row. One row
per (repository, member); filter on principal_id. The user-plane read behind
GET /v1/you/sync/plan and the console repository list — a tenant-addressed
equivalent would force the caller to already know the answer it is asking for.

A DISABLED membership yields nothing: losing access to a tenant must remove its
repositories from your list, not merely stop new ones appearing.

sync_enabled is TRUE throughout phase 1 (nothing gates yet) and becomes the
can_sync predicate in phase 2; denied_reason is NULL until then. Both are present
now so the plan endpoint reads the same columns before and after the gate.';
