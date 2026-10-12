-- Who reads the audit trail (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Client work and compliance").
--
-- | data                                   | lead (tenant role) | admin    |
-- |----------------------------------------|--------------------|----------|
-- | audit ledger (confidentiality actions) | R tenant           | R tenant |
-- | audit events (full)                    | —                  | R tenant |
--
-- Nobody writes audit rows from their own session: the row is the system's
-- record of what a person did (the matrix's service-role table).
--
-- The confidentiality actions are named ONCE, by dojo.is_confidentiality_action;
-- the lead's read policy and the dojo.confidentiality_ledger view both use it.
--
-- WHAT BREAKS THIS TEST: a lead policy without the action filter; an admin
-- clause at lead; a policy without the tenant; a ledger view that does not
-- filter; any INSERT grant to authenticated.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000081', 'a1000000-0000-4000-8000-000000000081', 'lea'),
  ('a0000000-0000-4000-8000-000000000082', 'a1000000-0000-4000-8000-000000000082', 'ada'),
  ('a0000000-0000-4000-8000-000000000083', 'a1000000-0000-4000-8000-000000000083', 'mia'),
  ('a0000000-0000-4000-8000-000000000084', 'a1000000-0000-4000-8000-000000000084', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000008a', 'organization/ztest-audit-t', 'organization',
   'ztest-audit-t', 'audit fixture T', 'dojo.sensei-hq.org/organization/ztest-audit-t'),
  ('b0000000-0000-4000-8000-00000000008b', 'organization/ztest-audit-u', 'organization',
   'ztest-audit-u', 'audit fixture U', 'dojo.sensei-hq.org/organization/ztest-audit-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000008a', 'a0000000-0000-4000-8000-000000000081', 'lead',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000008a', 'a0000000-0000-4000-8000-000000000082', 'admin',      'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000008a', 'a0000000-0000-4000-8000-000000000083', 'maintainer', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000008b', 'a0000000-0000-4000-8000-000000000084', 'admin',      'employer', 'github_oauth');

-- ztest-1 publish (T) · ztest-2 member_added (T) · ztest-3 contained (T) · ztest-4 publish (U)
insert into dojo.audit_events (tenant_id, action, target) values
  ('b0000000-0000-4000-8000-00000000008a', 'publish',      'ztest-1'),
  ('b0000000-0000-4000-8000-00000000008a', 'member_added', 'ztest-2'),
  ('b0000000-0000-4000-8000-00000000008a', 'contained',    'ztest-3'),
  ('b0000000-0000-4000-8000-00000000008b', 'publish',      'ztest-4');

set local role authenticated;

create temp table expect (who text, sub text, events text, ledger text) on commit drop;
insert into expect values
  ('lea', 'a1000000-0000-4000-8000-000000000081', '1,3',   '1,3'),
  ('ada', 'a1000000-0000-4000-8000-000000000082', '1,2,3', '1,3'),
  ('mia', 'a1000000-0000-4000-8000-000000000083', '',      ''),
  ('uma', 'a1000000-0000-4000-8000-000000000084', '4',     '4');

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(target, 1), ',' order by target), '') into got
          from dojo.audit_events where target like 'ztest-_';
        if got is distinct from r.events then
            raise exception 'audit_events RLS: % should see [%], saw [%]', r.who, r.events, got;
        end if;
        select coalesce(string_agg(right(target, 1), ',' order by target), '') into got
          from dojo.confidentiality_ledger where target like 'ztest-_';
        if got is distinct from r.ledger then
            raise exception 'confidentiality_ledger: % should see [%], saw [%]', r.who, r.ledger, got;
        end if;
    end loop;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000082"}', true);
    begin
        insert into dojo.audit_events (tenant_id, action, target)
        values ('b0000000-0000-4000-8000-00000000008a', 'publish', 'ztest-forged');
        raise exception 'audit_events: an admin wrote an audit row from their own session';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'audit_events: the write must be refused by the grant, not only a policy'; end if;
    end;
end $$;

rollback;
