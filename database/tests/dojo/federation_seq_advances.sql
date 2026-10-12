-- A federation pull cursor must see every WRITE, not only every insert.
--
-- `dojo.artifacts.seq` and `dojo.shared_rules.seq` are the cursors a daemon
-- resumes from (`seq > since`). Both were assigned only on INSERT, so:
--   - an artifact promoted submitted → published kept its insert-time seq, and
--     reached pullers only because the pull returned every status — which is
--     what let every member read other members' unpublished work (#236);
--   - a rule tombstoned or republished kept its seq, so a puller past it never
--     learned of the retraction.
-- An UPDATE now advances seq, as `dojo.relay_inbox_bump_seq` already does.
--
-- WHAT BREAKS THIS TEST: dropping either update trigger.

begin;

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000003a', 'organization/ztest-seq', 'organization',
   'ztest-seq', 'seq fixture', 'dojo.sensei-hq.org/organization/ztest-seq');
insert into sensei.namespaces (id, scope_key, name, slug) values
  ('e0000000-0000-4000-8000-0000000000b1', 'organization', 'ztest seq ns', 'ztest-seq-ns');

insert into dojo.artifacts (id, tenant_id, kind, title, body, signature) values
  ('70000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000003a', 'pattern', 'a', 'b', 'sig-a'),
  ('70000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000003a', 'pattern', 'later', 'b', 'sig-b');
insert into dojo.shared_rules (id, tenant_id, namespace_id, content_hash, rule_type, title, content,
                               enforcement, published_by, published_at) values
  ('f0000000-0000-4000-8000-0000000000b1', 'b0000000-0000-4000-8000-00000000003a',
   'e0000000-0000-4000-8000-0000000000b1', 'hs1', 'rule', 't', 'x', 'advisory', 'p', now()),
  ('f0000000-0000-4000-8000-0000000000b2', 'b0000000-0000-4000-8000-00000000003a',
   'e0000000-0000-4000-8000-0000000000b1', 'hs2', 'rule', 'later', 'x', 'advisory', 'p', now());

do $$
declare
    newest bigint;
    after  bigint;
begin
    -- The artifact inserted FIRST is published LAST: it must move past every
    -- row a puller may already have seen.
    select max(seq) into newest from dojo.artifacts where tenant_id = 'b0000000-0000-4000-8000-00000000003a';
    update dojo.artifacts set status = 'published' where id = '70000000-0000-4000-8000-000000000001';
    select seq into after from dojo.artifacts where id = '70000000-0000-4000-8000-000000000001';
    if after <= newest then
        raise exception 'artifacts: publishing must advance seq past %, got %', newest, after;
    end if;

    select max(seq) into newest from dojo.shared_rules where tenant_id = 'b0000000-0000-4000-8000-00000000003a';
    update dojo.shared_rules set status = 'tombstoned' where id = 'f0000000-0000-4000-8000-0000000000b1';
    select seq into after from dojo.shared_rules where id = 'f0000000-0000-4000-8000-0000000000b1';
    if after <= newest then
        raise exception 'shared_rules: a retraction must advance seq past %, got %', newest, after;
    end if;
end $$;

rollback;
