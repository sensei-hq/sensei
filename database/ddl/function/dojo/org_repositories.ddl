set search_path to dojo, extensions;

-- The org's repositories for its admin — the read behind
-- GET /v1/t/{origin}/{org}/repositories (the billing tab's Repositories
-- section). One row per repository with a standing (test 3's rule: no
-- standing, no row — the mockup lists no zero-member rows), ordered by
-- name, carrying the verdict columns the tab renders without inferring.
--
-- GRAIN + ACCESS (D3). The caller must hold admin in the tenant: the
-- route enforces ACCESS.admin before SQL, and this function asserts it
-- again (defense-in-depth for direct RPC and the demote-mid-request
-- race). The function stays total otherwise: a personal tenant (the org
-- tab does not exist there) or an unknown id returns empty, never raises.
--
-- SHARED VERDICTS (D1). Capture + entitlement come from
-- dojo.repo_entitlement() — the same call the member grain makes — never
-- a second CASE. Election + provenance are the view's org-authority
-- branches verbatim (the user branch is unreachable: personal tenants
-- return empty above). member_count joins exactly as
-- dojo.repo_role_at_least joins (Q1). last_synced_at reads every scope:
-- the caller is admin, and the management branch of
-- can_read_repository_metric admits user-scoped rows to admins — no
-- principal predicate, unlike the member read.
create or replace function dojo.org_repositories(p_tenant_id uuid)
    returns table (
        repository_id uuid,
        repo_key text,
        name text,
        provider dojo.forge_provider,
        forge_visibility text,
        member_count integer,
        last_synced_at timestamptz,
        elected boolean,
        configurable_by_me boolean,
        configured_by dojo.share_authority,
        configured_at timestamptz,
        entitlement_refusal text,
        billing_hidden boolean
    )
    language plpgsql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_origin dojo.tenant_origin;
begin
    if not dojo.tenant_role_at_least(p_tenant_id, 'admin') then
        raise exception 'org_repositories: admin role required for tenant %', p_tenant_id;
    end if;

    select t.origin into v_origin from dojo.tenants t where t.id = p_tenant_id;
    if v_origin is null or v_origin = 'personal' then
        return;
    end if;

    return query
    with counted as (
        -- Q1, joined exactly as repo_role_at_least joins: an effective
        -- role (override when overridden, else the forge's) that is not
        -- null, with an active membership in the repo's dōjō. Tenant
        -- scoping rides the outer predicate; the join itself is verbatim.
        select rr.repository_id, count(*)::int as member_count
          from dojo.repository_roles rr
          join dojo.repositories r on r.id = rr.repository_id
          join dojo.memberships m
            on m.tenant_id = r.tenant_id
           and m.user_id = rr.principal_id
           and m.disabled_at is null
         where (case when rr.overridden then rr.override_role else rr.forge_role end) is not null
         group by rr.repository_id
    ),
    synced as (
        -- Every scope: the caller is admin (asserted above), and admins
        -- read user-scoped rows too. No principal predicate.
        select rm.repository_id, max(rm.pushed_at) as last_synced_at
          from dojo.repository_metrics rm
         where rm.tenant_id = p_tenant_id
         group by rm.repository_id
    )
    select r.id
         , r.repo_key
         , r.name
         , r.provider
         , e.forge_visibility
         , c.member_count
         , s.last_synced_at
         -- ELECTION, the view's org-authority branches verbatim. The
         -- leading null guard is load-bearing (see the view: without it
         -- the CASE falls through to a branch whose authority is NOBODY).
         -- The view's trailing user branch is unreachable here — personal
         -- tenants return empty above — so the else names the shape.
         , case when e.forge_visibility is null then false
                when t.origin = 'organization' and e.forge_visibility <> 'public'
                     then coalesce(oe.elected, p.private_repos_shared, false)
                when t.origin = 'organization'
                     then coalesce(oe.elected, true)
                -- Unreachable: personal tenants return empty above, and
                -- tenant_origin is exactly {personal, organization} — a
                -- future third origin would silently take elected = true.
                else true
           end as elected
         -- May the caller change it, right now? The view's CASE
         -- specialised: null visibility reads false even for admins (L2);
         -- the caller is admin (asserted) on an organization tenant, so a
         -- known visibility is always configurable. Personal never reaches
         -- here (empty above), which is the view's personal → true arm.
         , case when e.forge_visibility is null then false
                else true
           end as configurable_by_me
         -- WHO decided, and WHEN — the view's org branches verbatim. A
         -- private policy never configures a public repo: provenance on
         -- org-public reads the org row or nobody.
         , case when e.forge_visibility is null then null::dojo.share_authority
                when t.origin = 'organization' and e.forge_visibility <> 'public'
                     then case when oe.elected          is not null
                                    then 'organization'::dojo.share_authority
                               when p.private_repos_shared is not null
                                    then 'organization'::dojo.share_authority
                               else null::dojo.share_authority
                          end
                when t.origin = 'organization'
                     then case when oe.elected is not null
                                    then 'organization'::dojo.share_authority
                               else null::dojo.share_authority
                          end
                else null::dojo.share_authority -- unreachable, as elected
           end as configured_by
         , case when e.forge_visibility is null then null::timestamptz
                when t.origin = 'organization' and e.forge_visibility <> 'public'
                     then case when oe.elected          is not null then oe.elected_at
                               when p.private_repos_shared is not null then p.set_at
                               else null
                          end
                when t.origin = 'organization'
                     then case when oe.elected is not null then oe.elected_at
                               else null
                          end
                else null::timestamptz -- unreachable, as elected
           end as configured_at
         , e.entitlement_refusal
         -- isBillingHiddenRepo, term for term: organization + private +
         -- a billing refusal. Each comparison is coalesced: a NULL term
         -- must read false (a public row is never hidden), never NULL.
         , (coalesce(t.origin = 'organization', false)
            and coalesce(e.forge_visibility = 'private', false)
            and coalesce(e.entitlement_refusal in ('unclaimed', 'not_subscribed', 'subscription_expired'), false)
           ) as billing_hidden
      from dojo.repositories r
      join dojo.tenants t on t.id = r.tenant_id
      -- INNER: no standing, no row (test 3's rule).
      join counted c on c.repository_id = r.id
      left join synced s on s.repository_id = r.id
      left join dojo.billing_accounts b on b.tenant_id = t.id
      left join dojo.tenant_share_policy p on p.tenant_id = t.id
      left join dojo.repository_elections oe on oe.repository_id = r.id
                                           and oe.authority     = 'organization'
      cross join lateral dojo.repo_entitlement(
          r.visibility, r.visibility_captured_at, t.origin, t.claimed_at,
          b.tenant_id, b.status, b.period_start, b.period_end) e
     where r.tenant_id = p_tenant_id
     order by r.name;
end;
$$;

revoke all on function dojo.org_repositories(uuid) from public;
grant execute on function dojo.org_repositories(uuid) to authenticated;

comment on function dojo.org_repositories(uuid) is
'The organization tenant''s repositories for its admin, one row per standing with the tab''s verdict columns (member_count, last_synced_at, election, entitlement, billing_hidden). Admin-asserted; personal/unknown tenants read empty.';
