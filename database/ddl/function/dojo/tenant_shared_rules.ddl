set search_path to dojo, sensei, extensions;

-- A tenant's shared-rule deltas with each rule's namespace, for a member's daemon
-- (GET rules, #236 step 4). The rows are readable under shared_rules_read_tenant,
-- but their namespaces are not: sensei.namespaces names private projects and
-- other tenants, and no session reads it. So the pull is pinned here — members
-- only, the caller from dojo.current_principal_id(), this tenant's rules after
-- the cursor (tombstones included, so a puller learns of a retraction).
create or replace function dojo.tenant_shared_rules(p_tenant uuid, p_since bigint)
    returns table (
        id             uuid,
        seq            bigint,
        status         text,
        version        integer,
        content_hash   text,
        rule_type      text,
        title          text,
        content        text,
        impact         text,
        enforcement    sensei.enforcement,
        origin_repo    text,
        published_by   text,
        published_at   timestamptz,
        scope_key      text,
        namespace_slug text,
        namespace_name text
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
    select r.id, r.seq, r.status, r.version, r.content_hash, r.rule_type, r.title, r.content,
           r.impact, r.enforcement, r.origin_repo, r.published_by, r.published_at,
           n.scope_key, n.slug, n.name
      from dojo.shared_rules r
      join sensei.namespaces n on n.id = r.namespace_id
     where r.tenant_id = p_tenant
       and r.seq > p_since
     order by r.seq;
end;
$$;

revoke all on function dojo.tenant_shared_rules(uuid, bigint) from public;
grant execute on function dojo.tenant_shared_rules(uuid, bigint) to authenticated;

comment on function dojo.tenant_shared_rules(uuid, bigint) is
'The tenant''s shared rules with seq > p_since, in seq order, each with its
namespace''s scope_key, slug and name — the daemon''s rules pull. Members only;
42501 otherwise.';
