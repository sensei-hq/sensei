-- A member's daemon pulls its tenant's shared rules, with each rule's namespace
-- (GET rules, #236 step 4).
--
-- The pull carries every rule's namespace (scope_key, slug, name). No session
-- reads sensei.namespaces — a namespace names private projects and other
-- tenants — so the pull goes through dojo.tenant_shared_rules: members only,
-- this tenant's rules only, deltas after the cursor, tombstones included so a
-- puller learns of a retraction.
--
-- WHAT BREAKS THIS TEST: a missing, disabled-blind or sessionless membership
-- check; rules of another tenant; ignoring the cursor or the order; dropping
-- tombstoned rows; a namespace field taken from the wrong column.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000191', 'a1000000-0000-4000-8000-000000000191', 'ola');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000019a', 'organization/ztest-tsr', 'organization', 'ztest-tsr', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000019b', 'organization/ztest-tsr-other', 'organization', 'ztest-tsr-other', 'O', 'x'),
  ('b0000000-0000-4000-8000-00000000019c', 'organization/ztest-tsr-gone', 'organization', 'ztest-tsr-gone', 'G', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000019a', 'a0000000-0000-4000-8000-000000000191', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000019c', 'a0000000-0000-4000-8000-000000000191', 'contributor', 'employer', 'github_oauth', now());
insert into sensei.namespaces (id, scope_key, slug, name) values
  ('e0000000-0000-4000-8000-000000000191', 'organization', 'ztest-tsr', 'Org rules'),
  ('e0000000-0000-4000-8000-000000000192', 'project', 'ztest-tsr-secret', 'Other tenant project');
insert into dojo.shared_rules (id, seq, tenant_id, namespace_id, content_hash, rule_type, title, content,
                               enforcement, status, published_by, published_at) values
  ('f0000000-0000-4000-8000-000000000191', 900001, 'b0000000-0000-4000-8000-00000000019a',
   'e0000000-0000-4000-8000-000000000191', 'h1', 'rule', 'first', 'c', 'recommended', 'active', 'x', now()),
  ('f0000000-0000-4000-8000-000000000192', 900002, 'b0000000-0000-4000-8000-00000000019a',
   'e0000000-0000-4000-8000-000000000191', 'h2', 'rule', 'retracted', 'c', 'required', 'tombstoned', 'x', now()),
  ('f0000000-0000-4000-8000-000000000193', 900003, 'b0000000-0000-4000-8000-00000000019b',
   'e0000000-0000-4000-8000-000000000192', 'h3', 'rule', 'other tenant', 'c', 'required', 'active', 'x', now()),
  ('f0000000-0000-4000-8000-000000000194', 900004, 'b0000000-0000-4000-8000-00000000019c',
   'e0000000-0000-4000-8000-000000000191', 'h4', 'rule', 'disabled tenant', 'c', 'required', 'active', 'x', now());

set local role authenticated;

do $$
declare
    got text;
    t     constant uuid := 'b0000000-0000-4000-8000-00000000019a';
    other constant uuid := 'b0000000-0000-4000-8000-00000000019b';
    gone  constant uuid := 'b0000000-0000-4000-8000-00000000019c';
begin
    perform set_config('request.jwt.claims', '', true);
    begin
        perform dojo.tenant_shared_rules(t, 0);
        raise exception 'tenant_shared_rules: answered with no session';
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000191"}', true);

    select string_agg(title || '/' || status || '/' || seq || '/' || scope_key || ':' || namespace_slug || '/' || namespace_name,
                      ' ' order by ordinality)
      into got from dojo.tenant_shared_rules(t, 900000) with ordinality;
    if got is distinct from 'first/active/900001/organization:ztest-tsr/Org rules retracted/tombstoned/900002/organization:ztest-tsr/Org rules' then
        raise exception 'tenant_shared_rules(t, 900000) returned %', got;
    end if;

    -- Deltas after the cursor only. (Both reads keep the function's own order:
    -- the daemon advances its cursor by the last row it sees.)
    select string_agg(title, ' ' order by ordinality) into got
      from dojo.tenant_shared_rules(t, 900001) with ordinality;
    if got is distinct from 'retracted' then
        raise exception 'tenant_shared_rules(t, 900001) returned %', got;
    end if;

    begin
        perform dojo.tenant_shared_rules(other, 0);
        raise exception 'tenant_shared_rules: answered for a tenant ola is not in';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.tenant_shared_rules(gone, 0);
        raise exception 'tenant_shared_rules: answered through a disabled membership';
    exception when insufficient_privilege then null;
    end;
end $$;

rollback;
