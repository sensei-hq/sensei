-- The dōjō says workspace on the governance ladder too (#307 R3b, D16).
--
-- The ladder (`sensei.scopes`, `sensei.namespaces`) is deployed to the dōjō as
-- well as to the daemon, and the two exchange it: the daemon sends a rule's
-- `scope_key` and a folder's `scope_key:slug` pairs, and federation files a
-- pulled rule under the local namespace of the same key. So the `project` rung
-- becomes `workspace` here in the same release, every namespace on it carried
-- across with its id (a shared rule or a billing seat points at that id). The
-- artifact scope tag `{"project": …}` is the same word in jsonb: an artifact,
-- the queues it moves through and a decision's distribution follow.
--
-- Two halves, as in tests/sensei/workspaces_renamed.sql: the converged database
-- holds no `project` rung or tag; then a pre-rename shape is put back, the
-- apply/before hooks run over it, the rows are found under the new key — and a
-- second run changes nothing.
--
-- WHAT BREAKS THIS TEST: the seed still keying the rung `project`; a hook that
-- skips the dōjō's database, re-inserts namespaces instead of moving them (their
-- ids, and every shared rule and seat on them, lost), leaves an artifact's tag
-- behind, or is not a no-op once it has run.

begin;
set local client_min_messages = warning;

create temp view dojo_project_keys as
select 'ladder rung ' || key as what from sensei.scopes where key = 'project'
union all
select 'namespace ' || scope_key || ':' || slug from sensei.namespaces where scope_key = 'project'
union all
select 'artifact scope ' || id from dojo.artifacts where scope ? 'project'
union all
select 'upstream scope ' || id from dojo.upstream_queue where scope ? 'project'
union all
select 'downstream scope ' || id from dojo.downstream_inbox where scope ? 'project'
union all
select 'decision distribution ' || id from dojo.decisions where distribution_scope ? 'project';

do $$
declare
    left_over text;
begin
    select string_agg(what, E'\n  ' order by what) into left_over from dojo_project_keys;
    if left_over is not null then
        raise exception E'the dōjō still stores a project key:\n  %', left_over;
    end if;
    if not exists (select 1 from sensei.scopes where key = 'workspace' and level = 60) then
        raise exception 'the dōjō''s ladder has no workspace rung at level 60';
    end if;
end $$;

-- ── the ladder and the tags as an older dōjō holds them ──────────────────────
insert into sensei.scopes (key, name, level, shareable, description, modified_at)
select 'project', 'Project', level, shareable, 'Rules for one logical project.', '2026-06-05'
  from sensei.scopes where key = 'workspace';
update sensei.namespaces set scope_key = 'project' where scope_key = 'workspace';
delete from sensei.scopes where key = 'workspace';

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000307b1', 'organization/ztest-r3b', 'organization', 'ztest-r3b', 'T', 'x');
insert into sensei.namespaces (id, scope_key, slug, name) values
  ('e0000000-0000-4000-8000-0000000307b1', 'project', 'ztest-r3b', 'ztest-r3b');
insert into dojo.shared_rules (id, seq, tenant_id, namespace_id, content_hash, rule_type, title, content,
                               enforcement, status, published_by, published_at) values
  ('f0000000-0000-4000-8000-0000000307b1', 930701, 'b0000000-0000-4000-8000-0000000307b1',
   'e0000000-0000-4000-8000-0000000307b1', 'h-r3b', 'rule', 'ztest-r3b', 'c', 'recommended', 'active', 'x', now());
insert into dojo.artifacts (id, tenant_id, kind, title, body, scope, signature) values
  ('c0000000-0000-4000-8000-0000000307b1', 'b0000000-0000-4000-8000-0000000307b1', 'principle',
   'ztest-r3b', 'b', '{"project": "ztest-r3b", "stack": "rust"}', 'sig-r3b');

create temp table counts_before as
select (select count(*) from sensei.namespaces where scope_key = 'project') as workspace_namespaces
     , (select count(*) from dojo.shared_rules) as shared_rules;

\ir ../../apply/before/dojo_workspaces_renamed.sql
\ir ../../apply/before/sensei_workspaces_renamed.sql

do $$
declare
    before counts_before;
    left_over text;
begin
    select * into before from counts_before;
    select string_agg(what, E'\n  ' order by what) into left_over from dojo_project_keys;
    if left_over is not null then
        raise exception E'the hooks left a project key behind:\n  %', left_over;
    end if;
    if (select count(*) from sensei.namespaces where scope_key = 'workspace') <> before.workspace_namespaces then
        raise exception 'namespaces on the project rung did not all move to workspace';
    end if;
    if (select count(*) from dojo.shared_rules) <> before.shared_rules
       or not exists (select 1 from dojo.shared_rules r
                        join sensei.namespaces n on n.id = r.namespace_id
                       where r.id = 'f0000000-0000-4000-8000-0000000307b1'
                         and n.scope_key = 'workspace' and n.slug = 'ztest-r3b') then
        raise exception 'the shared rule on the fixture namespace is not found under workspace';
    end if;
    if not exists (select 1 from dojo.artifacts
                    where id = 'c0000000-0000-4000-8000-0000000307b1'
                      and scope = '{"workspace": "ztest-r3b", "stack": "rust"}'::jsonb) then
        raise exception 'an artifact''s project scope tag did not become workspace';
    end if;
end $$;

-- ── a second run is a no-op ──────────────────────────────────────────────────
create temp table after_first as
select (select count(*) from sensei.scopes) as scopes
     , (select count(*) from sensei.namespaces where scope_key = 'workspace') as workspace_namespaces;

\ir ../../apply/before/dojo_workspaces_renamed.sql
\ir ../../apply/before/sensei_workspaces_renamed.sql

do $$
begin
    if (select count(*) from sensei.scopes) <> (select scopes from after_first)
       or (select count(*) from sensei.namespaces where scope_key = 'workspace')
          <> (select workspace_namespaces from after_first) then
        raise exception 'a second run of the hooks changed the ladder';
    end if;
end $$;

rollback;
