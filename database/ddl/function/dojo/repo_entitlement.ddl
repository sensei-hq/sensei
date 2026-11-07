set search_path to dojo, extensions;

-- The sharing verdict's capture + entitlement CASEs, shared by grain (D1).
--
-- Called by dojo.all_my_repositories (member grain) and
-- dojo.org_repositories (org grain). The CASE inputs are all
-- tenant/repo-level — no member input — so the verdict is computable at
-- either grain, and computing it twice is the exact failure
-- all_my_repositories' header documents ("WHY ONE VIEW AND NOT FOUR
-- DERIVATIONS"). Test 7 (parity, every code, both grains) is the lock.
--
-- THE TTL IS A LITERAL HERE AND NOWHERE ELSE: this function is the one
-- place that defines "a capture we still believe". Promoting it to a
-- settings row is worth doing when a second consumer needs it; a second
-- copy is not. (Moved here from the view with the CASEs, 2026-11-06.)
--
-- PURE over its arguments: reads no tables, so SECURITY INVOKER (nothing
-- to escalate) — but STABLE, not immutable: the period terms read now().
create or replace function dojo.repo_entitlement(
    p_visibility text,
    p_visibility_captured_at timestamptz,
    p_origin dojo.tenant_origin,
    p_claimed_at timestamptz,
    p_billing_tenant_id uuid,
    p_billing_status dojo.billing_status,
    p_period_start date,
    p_period_end date
)
    returns table (
        forge_visibility text,
        capture_refusal text,
        entitlement_refusal text
    )
    language sql
    stable
    security invoker
    set search_path = dojo, pg_temp
as $$
    with capture as (
        select -- THE FORGE'S ANSWER, and only if we can still stand behind it.
               --
               -- Three input states collapse to one NULL here — never captured,
               -- captured with no timestamp, and captured too long ago — because
               -- every consumer must treat them identically: no authority, no
               -- election, no sync.
               --
               -- WHY A STALE CAPTURE IS NOT MERELY A LAG. `public → free` fires
               -- above every billing term. A repository that goes private upstream
               -- would otherwise keep syncing free, under a user authority the
               -- organisation no longer holds — a confidentiality AND a billing
               -- bypass, for as long as nobody signs in. The TTL bounds that
               -- window to a number we can argue about instead of leaving it open.
               case when p_visibility_captured_at is null                      then null
                    when p_visibility_captured_at < now() - interval '30 days' then null
                    else p_visibility
               end as forge_visibility
               -- Which of the two capture failures happened, for the entitlement
               -- CASE. Total by construction: whenever `forge_visibility` is NULL
               -- this is not.
             , case when p_visibility             is null                      then 'forge_visibility_unknown'
                    when p_visibility_captured_at is null                      then 'forge_visibility_unknown'
                    when p_visibility_captured_at < now() - interval '30 days' then 'forge_visibility_stale'
                    else null
               end as capture_refusal
    )
    select c.forge_visibility
         , c.capture_refusal
         -- ENTITLEMENT, as a CODE rather than a boolean. A boolean cannot say
         -- WHICH of the reasons refused, which leaves `precedence` nothing to
         -- order and makes remedies interchangeable.
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
         -- NOTE what IS here: `origin = 'personal' → ALLOW`, below the capture
         -- term and above billing (D46). Entitlement keys on visibility for
         -- organisations, on ownership for personal tenants: a solo
         -- developer's own private repository is free, and a personal billing
         -- row — present or not, in any state — refuses nothing.
         --
         -- `claimed_at` is live and its term is enabled below. There is NO seat
         -- term, and there should not be (see all_my_repositories' header
         -- history: the seat model keys on the wrong concept; re-keying it is
         -- its own slice).
         , case
             when c.forge_visibility is null              then c.capture_refusal
             when c.forge_visibility = 'public'           then null   -- entitled, free
             -- A personal dōjō is free (D46): the person IS the tenant, so no
             -- subscription is owed. BELOW the capture term and above
             -- everything billing.
             when p_origin = 'personal'                   then null   -- entitled, free
             -- ABOVE the billing terms, and that ordering is the point: an
             -- unclaimed tenant CANNOT hold a billing account, so testing
             -- billing first would always answer `not_subscribed`, leaving
             -- `unclaimed` unreachable dead registry copy.
             --
             -- PERSONAL tenants are never unclaimed: they ARE the person, and
             -- there is nobody else to prove ownership against.
             when p_origin = 'organization'
              and p_claimed_at is null                    then 'unclaimed'
             when p_billing_tenant_id is null             then 'not_subscribed'
             when p_period_start is null
               or p_period_end   is null                  then 'not_subscribed'
             -- `trialing` IS a subscription. Excluding it demos the product
             -- with its core proposition switched off.
             when p_billing_status
                  not in ('active', 'trialing')            then 'not_subscribed'
             -- HALF-OPEN, deliberately. `period_end` is a DATE, so `between`
             -- casts it to midnight and denies the whole final day.
             when now() <  p_period_start
               or now() >= (p_period_end + 1)             then 'subscription_expired'
             else null                                                -- entitled
           end as entitlement_refusal
      from capture c
$$;

revoke all on function dojo.repo_entitlement(text, timestamptz, dojo.tenant_origin, timestamptz, uuid, dojo.billing_status, date, date) from public;
-- authenticated (direct/RPC use + the org read's session grain) and
-- service_role: all_my_repositories runs security_invoker, and the Worker
-- reads that view as service_role, so it needs execute on this function.
grant execute on function dojo.repo_entitlement(text, timestamptz, dojo.tenant_origin, timestamptz, uuid, dojo.billing_status, date, date) to authenticated, service_role;

comment on function dojo.repo_entitlement(text, timestamptz, dojo.tenant_origin, timestamptz, uuid, dojo.billing_status, date, date) is
'The capture + entitlement verdict over tenant/repo-level inputs (D1): forge_visibility (TTL''d), capture_refusal, entitlement_refusal. One home for both grains — all_my_repositories and org_repositories call it; test 7 locks the parity.';
