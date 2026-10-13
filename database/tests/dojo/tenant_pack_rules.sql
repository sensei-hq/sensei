-- The rules of packs a tenant adopted, resolved for a member's daemon
-- (GET rules/resolved, #236 step 4; decision 2026-10-12: tenant-owned namespaces).
--
-- The daemon names its folder's namespaces as (scope_key, slug) pairs. A member
-- of the path tenant gets the packs adopted at the namespaces that TENANT owns —
-- `organization:<slug>` for an organization tenant, `team:<slug>` for a team —
-- each rule at the adoption's scope, its tier raised (never lowered) by the
-- adoption's override. Every other namespace the daemon names is dropped: no
-- tenant owns a technology, repository, project or personal namespace, and
-- another tenant's is not this member's to read.
--
-- WHAT BREAKS THIS TEST: a membership check that is missing, ignores a disabled
-- membership, or passes with no session; ownership matched on the slug alone (a
-- team owning the organization namespace of the same name) or on the scope
-- alone; ignoring the daemon's pairs (a namespace the folder does not name);
-- an override that weakens a rule, or is dropped.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000181', 'a1000000-0000-4000-8000-000000000181', 'ola');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000018a', 'organization/ztest-tpr', 'organization', 'ztest-tpr', 'Org', 'x'),
  ('b0000000-0000-4000-8000-00000000018b', 'team/ztest-tpr-team', 'team', 'ztest-tpr-team', 'Team', 'x'),
  ('b0000000-0000-4000-8000-00000000018c', 'organization/ztest-tpr-other', 'organization', 'ztest-tpr-other', 'Other', 'x'),
  ('b0000000-0000-4000-8000-00000000018d', 'organization/ztest-tpr-gone', 'organization', 'ztest-tpr-gone', 'Gone', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000018a', 'a0000000-0000-4000-8000-000000000181', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000018b', 'a0000000-0000-4000-8000-000000000181', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000018d', 'a0000000-0000-4000-8000-000000000181', 'contributor', 'employer', 'github_oauth', now());

insert into sensei.namespaces (id, scope_key, slug, name) values
  ('e0000000-0000-4000-8000-000000000181', 'organization', 'ztest-tpr', 'org'),
  ('e0000000-0000-4000-8000-000000000182', 'team', 'ztest-tpr-team', 'team'),
  ('e0000000-0000-4000-8000-000000000183', 'organization', 'ztest-tpr-other', 'other'),
  ('e0000000-0000-4000-8000-000000000184', 'technology', 'ztest-tpr-react', 'react'),
  ('e0000000-0000-4000-8000-000000000185', 'organization', 'ztest-tpr-team', 'same name as the team'),
  ('e0000000-0000-4000-8000-000000000186', 'organization', 'ztest-tpr-gone', 'gone');

insert into sensei.rule_packs (id, slug, name, area, attribution, published_by) values
  ('f0000000-0000-4000-8000-000000000181', 'ztest-tpr-org', 'Org pack', 'security', 'OWASP', 'x'),
  ('f0000000-0000-4000-8000-000000000182', 'ztest-tpr-team', 'Team pack', 'process', 'Team', 'x'),
  ('f0000000-0000-4000-8000-000000000183', 'ztest-tpr-other', 'Other pack', 'process', 'Other', 'x'),
  ('f0000000-0000-4000-8000-000000000184', 'ztest-tpr-react', 'React pack', 'tech_stack', 'React', 'x'),
  ('f0000000-0000-4000-8000-000000000185', 'ztest-tpr-samename', 'Same-name pack', 'process', 'Same', 'x'),
  ('f0000000-0000-4000-8000-000000000186', 'ztest-tpr-gone', 'Gone pack', 'process', 'Gone', 'x');
insert into sensei.rule_pack_rules (pack_id, ordinal, statement, enforcement) values
  ('f0000000-0000-4000-8000-000000000181', 1, 'org advisory',   'advisory'),
  ('f0000000-0000-4000-8000-000000000181', 2, 'org mandatory',  'mandatory'),
  ('f0000000-0000-4000-8000-000000000182', 1, 'team rule',      'recommended'),
  ('f0000000-0000-4000-8000-000000000183', 1, 'other rule',     'required'),
  ('f0000000-0000-4000-8000-000000000184', 1, 'react rule',     'required'),
  ('f0000000-0000-4000-8000-000000000185', 1, 'same-name rule', 'required'),
  ('f0000000-0000-4000-8000-000000000186', 1, 'gone rule',      'required');
insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, adopted_by, enforcement) values
  ('f0000000-0000-4000-8000-000000000181', 'e0000000-0000-4000-8000-000000000181', 1, 'x', 'required'),
  ('f0000000-0000-4000-8000-000000000182', 'e0000000-0000-4000-8000-000000000182', 1, 'x', null),
  ('f0000000-0000-4000-8000-000000000183', 'e0000000-0000-4000-8000-000000000183', 1, 'x', null),
  ('f0000000-0000-4000-8000-000000000184', 'e0000000-0000-4000-8000-000000000184', 1, 'x', null),
  ('f0000000-0000-4000-8000-000000000185', 'e0000000-0000-4000-8000-000000000185', 1, 'x', null),
  ('f0000000-0000-4000-8000-000000000186', 'e0000000-0000-4000-8000-000000000186', 1, 'x', null);

set local role authenticated;

do $$
declare
    got text;
    n int;
    org   constant uuid := 'b0000000-0000-4000-8000-00000000018a';
    team  constant uuid := 'b0000000-0000-4000-8000-00000000018b';
    other constant uuid := 'b0000000-0000-4000-8000-00000000018c';
    gone  constant uuid := 'b0000000-0000-4000-8000-00000000018d';
    every constant text[] := array['organization:ztest-tpr', 'team:ztest-tpr-team', 'organization:ztest-tpr-other',
                                   'technology:ztest-tpr-react', 'organization:ztest-tpr-team',
                                   'organization:ztest-tpr-gone', 'user:a0000000-0000-4000-8000-000000000181'];
begin
    -- No session: nobody to check membership for.
    perform set_config('request.jwt.claims', '', true);
    begin
        perform dojo.tenant_pack_rules(org, every);
        raise exception 'tenant_pack_rules: answered with no session';
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000181"}', true);

    -- The organization's own namespace only, the override raising the advisory
    -- rule and leaving the mandatory one where it is.
    select string_agg(statement || '/' || enforcement || '/' || scope_key || '/' || attribution, ' ' order by ordinal)
      into got from dojo.tenant_pack_rules(org, every);
    if got is distinct from 'org advisory/required/organization/OWASP org mandatory/mandatory/organization/OWASP' then
        raise exception 'tenant_pack_rules(org) returned %', got;
    end if;

    -- The team owns team:<slug>, not the organization namespace of the same name.
    select string_agg(statement || '/' || enforcement || '/' || scope_key, ' ')
      into got from dojo.tenant_pack_rules(team, every);
    if got is distinct from 'team rule/recommended/team' then
        raise exception 'tenant_pack_rules(team) returned %', got;
    end if;

    -- Only what the daemon's folder names.
    select count(*) into n from dojo.tenant_pack_rules(org, array['technology:ztest-tpr-react', 'team:ztest-tpr-team']);
    if n <> 0 then raise exception 'tenant_pack_rules: resolved % rules at namespaces the folder did not name for its tenant', n; end if;

    begin
        perform dojo.tenant_pack_rules(other, every);
        raise exception 'tenant_pack_rules: answered for a tenant ola is not in';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.tenant_pack_rules(gone, every);
        raise exception 'tenant_pack_rules: answered through a disabled membership';
    exception when insufficient_privilege then null;
    end;
end $$;

rollback;
