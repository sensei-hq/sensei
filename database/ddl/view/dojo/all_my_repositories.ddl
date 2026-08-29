set search_path to dojo, extensions;

-- "Which repositories are mine, may each be shared, did anyone choose to share
--  it — and if not, why, and what do I do about it?"
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
-- SHARING IS TWO QUESTIONS, AND THIS VIEW ANSWERS BOTH.
--
--   may_share  ENTITLEMENT — may it be shared at all? Keys on the FORGE's
--              visibility and the tenant's subscription. Never on who owns the
--              tenant: `public` is free for anyone, `private` is paid for by
--              everyone, including a solo developer's own repository.
--   elected    ELECTION — did whoever holds authority actually choose it?
--   sync_enabled = may_share AND elected
--
-- Keeping them apart is the whole point. "Allowed but nobody chose" and "chosen
-- but not allowed" are different states with different fixes, and one boolean
-- cannot tell a user which they are in — which is how `sync_enabled = true` came
-- to be hardcoded here in phase 1.
--
-- WHY ONE VIEW AND NOT FOUR DERIVATIONS. The sync decision was being re-derived
-- in at least four places (the daemon's gate, the plan endpoint, the console, and
-- any UI greying out a toggle). Four derivations drift, and when they disagree
-- nobody can say which is right. The daemon, the API and the UI read this, so a
-- disagreement becomes impossible rather than merely unlikely.
--
-- THE REGISTRY DECORATES; IT NEVER GATES. `sensei.reason_codes` (through
-- `dojo.reason_codes`) holds no predicate and no threshold — delete every row and
-- the same repositories sync, losing only the prose. So every verdict below is
-- computed from DOMAIN data and the join is LEFT, with `coalesce(rc.summary,
-- reason_code)`: a code the registry has not translated must surface RAW, never
-- remove the repository from a sync-decision view.
--
-- SECURITY: `security_invoker = on` — runs as the calling role, not the owner
-- (Supabase lint security_definer_view). Read server-side as service_role; the
-- caller supplies the principal filter. Not granted to any client role, so a
-- user's repository list is never exposed via PostgREST.
--
-- KNOWN RISK, stated rather than discovered: service_role BYPASSES RLS whatever
-- `security_invoker` says, so for the Worker the entire boundary is the app's
-- `.eq('principal_id', …)`. One consumer forgetting it exposes every tenant's
-- members, elections and billing state at once. The structural fix — wrapping
-- this in a SECURITY DEFINER function taking the principal as a REQUIRED
-- argument — is the next slice, not review discipline.
--
-- See docs/spec/dojo/daemon-sync.md §8a/§8b/§8c and
-- docs/requirements/repository-sharing.md.
--
-- Recreated rather than replaced: `create or replace view` cannot rename
-- `visibility` → `forge_visibility` nor insert columns before `sync_enabled`.
drop view if exists dojo.all_my_repositories;

create view dojo.all_my_repositories
with (security_invoker = on)
as
with member_repo as (
    select r.id                as repository_id
         , r.repo_key
         , r.name
         , r.remote_url
         , r.provider
         , t.id                as tenant_id
         , t.key               as tenant        -- the discovery path <origin>/<slug>
         , t.slug              as owning_org
         , t.origin
         , m.user_id           as principal_id  -- whose row this is; the filter column
         , m.role

         -- THE FORGE'S ANSWER, and only if we can still stand behind it.
         --
         -- Three input states collapse to one NULL here — never captured, captured
         -- with no timestamp, and captured too long ago — because every consumer
         -- must treat them identically: no authority, no election, no sync. The
         -- normalization happens ONCE, and every derived column below then carries
         -- its OWN leading `forge_visibility is null` guard.
         --
         -- WHY A STALE CAPTURE IS NOT MERELY A LAG. `public → free` fires above
         -- every billing term. A repository that goes private upstream would
         -- otherwise keep syncing free, under a user authority the organisation no
         -- longer holds — a confidentiality AND a billing bypass, for as long as
         -- nobody signs in. The TTL bounds that window to a number we can argue
         -- about instead of leaving it open.
         --
         -- The TTL is a literal HERE and nowhere else: it is the one place that
         -- defines "a capture we still believe". Promoting it to a settings row is
         -- worth doing when a second consumer needs it; a second copy is not.
         , case when r.visibility_captured_at is null                          then null
                when r.visibility_captured_at < now() - interval '30 days'     then null
                else r.visibility
           end                 as forge_visibility

         -- Which of the two capture failures happened, for the entitlement CASE.
         -- Total by construction: whenever `forge_visibility` is NULL this is not.
         , case when r.visibility             is null                          then 'forge_visibility_unknown'
                when r.visibility_captured_at is null                          then 'forge_visibility_unknown'
                when r.visibility_captured_at < now() - interval '30 days'     then 'forge_visibility_stale'
                else null
           end                 as capture_refusal

         -- Entitlement inputs. `billing_tenant_id` is how a MISSING row is told
         -- from a row whose status is unset — the distinction the whole gate turned
         -- on (see the CASE below).
         , b.tenant_id         as billing_tenant_id
         , b.status            as billing_status
         , b.period_start
         , b.period_end

         -- Election inputs. Absent row = NOT elected, never elected-by-default.
         , p.private_repos_shared
         , p.set_at            as policy_set_at
         , oe.elected          as org_elected
         , oe.elected_at       as org_elected_at
         , ue.elected          as user_elected
         , ue.elected_at       as user_elected_at

         , mx.last_synced_at
         , mx.metric_rows
      from dojo.repositories r
      join dojo.tenants t     on t.id = r.tenant_id
      join dojo.memberships m on m.tenant_id = r.tenant_id
                            and m.disabled_at is null
      left join dojo.billing_accounts b    on b.tenant_id = t.id
      left join dojo.tenant_share_policy p on p.tenant_id = t.id
      -- The organisation's election for this repository: the per-repo exception a
      -- tenant-wide flag cannot express ("share all private repos except this
      -- one"). One row at most — `unique nulls not distinct (repository_id,
      -- authority, principal_id)` with a NULL principal for an org election.
      left join dojo.repository_elections oe on oe.repository_id = r.id
                                           and oe.authority     = 'organization'
      -- THIS member's own election. Scoped to the principal: another member's
      -- choice is not this member's, and a stale row left behind by an authority
      -- change is kept as history and simply not read.
      left join dojo.repository_elections ue on ue.repository_id = r.id
                                           and ue.authority     = 'user'
                                           and ue.principal_id  = m.user_id
      -- Aggregated in a LATERAL rather than inline: this view is one row per
      -- (repository, MEMBER), so a joined aggregate would multiply the rows.
      left join lateral (
          select max(rm.pushed_at) as last_synced_at
               , count(*)          as metric_rows
            from dojo.repository_metrics rm
           where rm.repository_id = r.id
             -- NOT every row. `repository_metrics` carries `principal_id` for
             -- scope='user' rows, and the Worker reads this view as service_role,
             -- which BYPASSES the RLS on that table — so this predicate is the
             -- whole boundary. Without it, member A's row shows member B's push
             -- timestamp and contribution volume on a repository A never elected.
             -- `dojo.can_read_repository_metric` states the rule verbatim: "metrics
             -- by user visible to every peer is surveillance, not transparency."
             and (rm.scope = 'repo' or rm.principal_id = m.user_id)
      ) mx on true
),
axes as (
    select mr.*

         -- AUTHORITY — derived from (origin, forge visibility), never stored on
         -- the repository. Authority follows who owns the code and who pays: an
         -- organisation's PRIVATE repositories are its own, on its own
         -- subscription; everything else — personal repos of either visibility,
         -- and an organisation's PUBLIC repos — is the member's call, because the
         -- org is not paying for open source and a contributor's metrics are their
         -- own.
         , case when mr.forge_visibility is null then null::dojo.share_authority
                when mr.origin = 'organization'
                 and mr.forge_visibility <> 'public' then 'organization'::dojo.share_authority
                else 'user'::dojo.share_authority
           end as authority

         -- ENTITLEMENT, as a CODE rather than a boolean. A boolean cannot say
         -- WHICH of the reasons refused, which leaves `precedence` nothing to
         -- order and makes remedies interchangeable — "ask an admin to subscribe"
         -- when the org already subscribes and the member simply has no seat.
         -- `may_share` is derived FROM this, below.
         --
         -- THE ORDER IS THE LOAD-BEARING PART:
         --
         --  * capture first — an unknown or expired forge answer refuses before
         --    anything else can consult it;
         --  * `public` next — open source is free to host, and it must sit ABOVE
         --    billing or an unsubscribed contributor's public repo would be
         --    refused for a subscription nobody owes;
         --  * the MISSING billing row BEFORE its status. `NULL <> 'active'` is
         --    NULL, not TRUE, so a value test alone falls through to ALLOW.
         --    Verified live: 3 tenants, 0 billing rows, and the composite ALLOWED
         --    an org-mandated private repository on no subscription at all.
         --
         -- NOTE what is NOT here: no `origin = 'personal' → ALLOW`. Entitlement
         -- keys on VISIBILITY, not on who owns the tenant. An earlier draft's
         -- unconditional personal-ALLOW would have hosted every personal private
         -- repository free — the common case, since no personal tenant carries a
         -- billing row.
         --
         -- PHASE 2 terms sit COMMENTED at their precedence positions.
         -- `dojo.tenants.claimed_at` and `dojo.seat_allocations` do not exist yet;
         -- writing them live would make this view unbuildable, and writing them
         -- elsewhere later would mean re-deriving the order.
         , case
             when mr.forge_visibility is null              then mr.capture_refusal
             when mr.forge_visibility = 'public'           then null   -- entitled, free
             -- PHASE 2, and it must stay ABOVE the billing terms: an unclaimed
             -- tenant CANNOT hold a billing account, so testing billing first would
             -- always answer `not_subscribed` — telling the reader to buy something
             -- the service refuses to sell until someone claims the org, and leaving
             -- `unclaimed` unreachable, i.e. dead registry copy.
             -- PHASE 2: when mr.claimed_at is null        then 'unclaimed'
             when mr.billing_tenant_id is null             then 'not_subscribed'
             when mr.period_start is null
               or mr.period_end   is null                  then 'not_subscribed'
             -- `trialing` IS a subscription. Excluding it demos the product with
             -- its core proposition switched off and tells the admin to buy what
             -- they are already trialling.
             when mr.billing_status
                  not in ('active', 'trialing')            then 'not_subscribed'
             -- HALF-OPEN, deliberately. `period_end` is a DATE, so `between` casts
             -- it to midnight and denies the whole final day — an org's sync stops
             -- a day early, announcing a lapse that has not happened.
             when now() <  mr.period_start
               or now() >= (mr.period_end + 1)             then 'subscription_expired'
             -- PHASE 2, and `released_at is null` will be load-bearing: allocations
             -- are kept as history rather than deleted, so a bare `is not null`
             -- matches a RELEASED seat and a departed employee keeps pushing after
             -- de-provisioning reported success.
             -- PHASE 2: when sa.id is null                then 'no_seat'
             else null                                                -- entitled
           end as entitlement_refusal

         -- ELECTION — the organisation's policy where it holds authority, this
         -- member's own election everywhere else.
         --
         -- THE LEADING GUARD IS NOT REDUNDANT with the one in `entitlement_refusal`
         -- and is not decoration. Without it, `'organization' = 'organization' and
         -- NULL <> 'public'` evaluates to NULL — not FALSE — so the ORG branch is
         -- NOT taken and the CASE falls through to the ELSE, reading the USER's
         -- election for a repository whose authority is NOBODY. The row would then
         -- report `authority = NULL` beside an election the user made, and
         -- `sync_enabled` would come out false only because a SIBLING column
         -- happened to guard the same condition. One column's correctness must
         -- never rest on another's. Verified in Postgres, not reasoned about.
         --
         -- `coalesce(…, false)` throughout, including on the tenant policy: an
         -- absent row is NOT ELECTED, never elected-by-default, and a NULL verdict
         -- is unusable to a consumer that has to decide.
         , case when mr.forge_visibility is null then false
                when mr.origin = 'organization' and mr.forge_visibility <> 'public'
                     then coalesce(mr.org_elected, mr.private_repos_shared, false)
                else coalesce(mr.user_elected, false)
           end as elected

         -- May THIS member change it, right now? The view is already
         -- per-principal, so this answers "should the UI show a toggle" directly.
         --
         -- ADMIN ONLY on org-private, not lead: what would be changed is the
         -- organisation's POLICY, and `dojo.member_role` assigns policy, identity
         -- and provisioning to `admin` — a lead guards confidentiality on client
         -- engagements. Every seeded remedy for an org-authority refusal reads "ask
         -- an admin"; letting a lead flip it would make that copy a lie.
         , case when mr.forge_visibility is null                          then false
                when mr.origin = 'personal'
                  or mr.forge_visibility = 'public'                       then true
                when mr.role = 'admin'                                    then true
                else false
           end as configurable_by_me

         -- WHO decided, and WHEN — for the authority that actually applies. A row
         -- recording `elected = false` is still a decision, and saying so is the
         -- difference between "your org turned this off" and "nobody has looked".
         , case when mr.forge_visibility is null then null::dojo.share_authority
                when mr.origin = 'organization' and mr.forge_visibility <> 'public'
                     then case when mr.org_elected          is not null
                                    then 'organization'::dojo.share_authority
                               when mr.private_repos_shared is not null
                                    then 'organization'::dojo.share_authority
                               else null::dojo.share_authority
                          end
                else case when mr.user_elected is not null
                               then 'user'::dojo.share_authority
                          else null::dojo.share_authority
                     end
           end as configured_by
         , case when mr.forge_visibility is null then null::timestamptz
                when mr.origin = 'organization' and mr.forge_visibility <> 'public'
                     then case when mr.org_elected          is not null then mr.org_elected_at
                               when mr.private_repos_shared is not null then mr.policy_set_at
                               else null
                          end
                else case when mr.user_elected is not null then mr.user_elected_at else null end
           end as configured_at
      from member_repo mr
),
verdict as (
    select a.*
         , (a.entitlement_refusal is null)               as may_share
         , (a.entitlement_refusal is null and a.elected) as sync_enabled

         -- WHICH QUESTION REFUSED. A separate column rather than something a
         -- consumer infers from the reason string: "pay the invoice", "ask an
         -- admin" and "turn it on yourself" all read as `sync_enabled = false`,
         -- and an inference gets it wrong the first time a reason is added.
         , case when a.entitlement_refusal is not null then 'entitlement'
                when not a.elected                    then 'election'
                else null
           end                                           as refused_by

         -- THE PRECEDENCE-ORDERED PICK. A repository can fail several ways at
         -- once; the one reported is the one to fix FIRST. Entitlement outranks
         -- election by construction — its codes occupy 10-40 and election's 50-51 —
         -- so "lowest precedence" and "entitlement first" are the same rule. The
         -- seed data must keep it that way; `unique (domain, precedence)` is what
         -- makes an accidental overlap fail loudly.
         , coalesce(
               a.entitlement_refusal
             , case when a.elected                     then null
                    when a.authority = 'organization'  then 'not_elected_org'
                    else                                    'not_elected_user'
               end
           )                                             as reason_code
      from axes a
)
select v.repository_id
     , v.repo_key
     , v.name
     , v.remote_url
     , v.provider
     , v.tenant_id
     , v.tenant
     , v.owning_org
     , v.origin
     , v.principal_id
     , v.role

     , v.forge_visibility
     , v.authority
     , v.may_share
     , v.elected
     , v.sync_enabled

     , v.entitlement_refusal
     , v.refused_by
     , v.reason_code
     -- LEFT JOIN + coalesce: an untranslated code surfaces raw rather than
     -- silently emptying the row that carries it.
     , coalesce(rc.summary, v.reason_code)               as reason
     , rc.detail                                         as reason_detail
     , rc.remedy
     , rc.actor                                          as reason_actor

     , v.configurable_by_me
     , v.configured_by
     , v.configured_at
     , v.last_synced_at
     , v.metric_rows

     -- DEPRECATED, and kept only until the plan endpoint reads `reason_code`:
     -- `GET /v1/you/sync/plan` still selects this column and puts it on the wire
     -- as `denied[].reason`, which is exactly `reason_code`. Removing it here
     -- before that consumer moves would turn every denial into `not_permitted`.
     , v.reason_code                                     as denied_reason
  from verdict v
  left join dojo.reason_codes rc on rc.domain = 'repository_sharing'
                               and rc.code   = v.reason_code;

comment on view dojo.all_my_repositories is
'Every repository a user can reach, with its owning tenant on the row, and the
complete sharing verdict for that (repository, member) pair. Filter on
principal_id. The user-plane read behind GET /v1/you/sync/plan and the console
repository list — a tenant-addressed equivalent would force the caller to already
know the answer it is asking for.

A DISABLED membership yields nothing: losing access to a tenant must remove its
repositories from your list, not merely stop new ones appearing.

TWO VERDICTS, never one boolean. may_share is ENTITLEMENT (may it be shared —
forge visibility and subscription, never who owns the tenant); elected is
ELECTION (did whoever holds authority choose it); sync_enabled is both. A refusal
always names refused_by (entitlement | election) and a reason_code, because "off"
that does not say which question refused sends the reader hunting for the wrong
fix.

AUTHORITY is derived, never stored: an organisation''s PRIVATE repositories are
the organisation''s (mandatory — the member cannot override it in either
direction); everything else is the member''s. An uncaptured or expired forge
visibility yields NO authority, NO election and NO sync.

reason/remedy/reason_actor are joined LEFT from dojo.reason_codes and are
REPORTING ONLY — no verdict here reads the registry, so deleting every row
changes the prose and nothing else.';

comment on column dojo.all_my_repositories.forge_visibility
     is 'The forge''s answer, or NULL when it was never captured, carries no capture timestamp, or is older than the capture TTL. All three mean the same thing to every consumer: no authority, no election, no sync.';
comment on column dojo.all_my_repositories.may_share
     is 'ENTITLEMENT. Keys on forge visibility and subscription — never on tenant origin. Public is free for anyone; private is subscription-gated for everyone, including a solo developer''s own repository.';
comment on column dojo.all_my_repositories.elected
     is 'ELECTION. The organisation''s policy where it holds authority, this member''s own election everywhere else. Absent row = not elected.';
comment on column dojo.all_my_repositories.refused_by
     is 'entitlement | election | NULL — which of the two questions said no. A column rather than an inference from the reason string, so a reason added later cannot land on the wrong side of the split.';
comment on column dojo.all_my_repositories.configurable_by_me
     is 'May THIS member change it right now? Admin-only on org-private repositories, because what changes is the organisation''s policy and member_role assigns policy to admin.';
comment on column dojo.all_my_repositories.last_synced_at
     is 'Latest push for this repository that THIS member may see: repo-scoped rows, plus their own user-scoped rows. Not every row — service_role bypasses the RLS on repository_metrics, so this predicate is the boundary.';
comment on column dojo.all_my_repositories.denied_reason
     is 'DEPRECATED alias of reason_code, kept until GET /v1/you/sync/plan reads reason_code directly.';
