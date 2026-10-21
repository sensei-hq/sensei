set search_path to dojo, sensei, extensions;

-- The rules of packs a tenant adopted, for a member's daemon (GET rules/resolved,
-- #236 step 4). sensei.rule_pack_adoptions spans every namespace and stays the
-- system's; a member reads their tenant's through this, the caller pinned to
-- dojo.current_principal_id().
--
-- A tenant OWNS the namespace its key names: `organization/<slug>` owns
-- `organization:<slug>` (a personal tenant owns none: there is no `personal`
-- namespace scope; and a dōjō is only ever one of the two, #307 D11). Derived,
-- never claimed — an organization tenant is forge-proven (decision 2026-10-12),
-- so nobody can own a namespace by naming it first. `p_namespaces` are the daemon folder's `scope_key:slug`
-- pairs; only the tenant's own among them resolve.
create or replace function dojo.tenant_pack_rules(p_tenant uuid, p_namespaces text[])
    returns table (
        pack_id      uuid,
        rule_id      uuid,
        ordinal      integer,
        statement    text,
        body         text,
        rationale    text,
        enforcement  sensei.enforcement,
        verification sensei.rule_check,
        checker_ref  text,
        remediation  text,
        skill_ref    text,
        applies_to   jsonb,
        attribution  text,
        area         sensei.rule_pack_area,
        scope_key    text
    )
    language plpgsql
    stable
    security definer
    set search_path = dojo, sensei, pg_temp
as $$
#variable_conflict use_column
begin
    if not dojo.is_member(p_tenant) then
        raise exception 'not a member of this tenant' using errcode = 'insufficient_privilege';
    end if;

    return query
    select r.pack_id, r.id, r.ordinal, r.statement, r.body, r.rationale,
           greatest(r.enforcement, coalesce(a.enforcement, r.enforcement)),
           r.verification, r.checker_ref, r.remediation, r.skill_ref, r.applies_to,
           p.attribution, p.area, n.scope_key
      from dojo.tenants t
      join sensei.namespaces n
        on n.scope_key = t.origin::text
       and n.slug = t.slug
       and n.scope_key || ':' || n.slug = any (p_namespaces)
      join sensei.rule_pack_adoptions a on a.namespace_id = n.id
      join sensei.rule_packs p on p.id = a.pack_id
      join sensei.rule_pack_rules r on r.pack_id = p.id
     where t.id = p_tenant
     order by p.slug, r.ordinal;
end;
$$;

revoke all on function dojo.tenant_pack_rules(uuid, text[]) from public;
grant execute on function dojo.tenant_pack_rules(uuid, text[]) to authenticated;

comment on function dojo.tenant_pack_rules(uuid, text[]) is
'Rules of the packs adopted at the namespace the tenant owns (organization:<slug>
or team:<slug>, from its key), among the daemon folder''s `scope_key:slug` pairs —
each at the adoption''s scope, its tier raised (never lowered) by the adoption''s
override. Members only; 42501 otherwise.';
