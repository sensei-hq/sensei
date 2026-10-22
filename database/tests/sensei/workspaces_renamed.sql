-- Sensei says workspace (#307 R3a, D12/D30): the daemon's own database renames
-- `sensei.projects` and everything that names it IN PLACE, keeping its rows.
--
-- Two halves. First, the converged schema: no sensei-owned relation, column,
-- type, constraint, index, function or enum label still says `project`. Second,
-- the upgrade: this file puts a pre-rename shape back (the tables, columns,
-- types, enum labels and constraint names the hook renames, with a row in
-- each), runs `apply/before/sensei_workspaces_renamed.sql` over it, and checks
-- the rows came through under the new names — then runs it again, which must
-- change nothing.
--
-- R3b adds the stored KEYS sensei names after a project: the retired metric
-- `project_health` and the setup wizard's `setup.projects` config key become
-- `workspace_health` / `setup.workspaces`, the row kept (the metric's id, so its
-- recorded values follow it). And the governance ladder's `project` rung (D16):
-- `sensei.scopes.key` becomes `workspace` with every namespace on it carried
-- across (their ids kept, so a stance or rule attached to one follows it), an
-- inbox artifact's scope tag `{"project": …}` becomes `{"workspace": …}`, and the
-- memory history's text copy of the scope follows the live enum.
--
-- WHAT BREAKS THIS TEST: a DDL file that still declares a `project` name; the
-- hook missing a table, column, type, enum label or constraint, so dbd would
-- recreate it empty or leave the old name behind; a rename that drops rows; a
-- hook that is not a no-op once it has run; a stored key still saying project,
-- or converted by inserting a fresh row instead of renaming the one there.
--
-- What keeps the word `project` is another system's: `mcp_servers.scope`
-- holds Claude Code's `'user' | 'project'` (a VALUE, not a name), the task
-- kind `compile_projects` compiles a compiler's projects.

begin;
set local client_min_messages = warning;

-- ── the converged schema says workspace ──────────────────────────────────────
create temp view sensei_project_names as
select 'relation ' || n.nspname || '.' || c.relname as what
  from pg_class c join pg_namespace n on n.oid = c.relnamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway')
   and c.relname ilike '%project%'
union all
select 'column ' || n.nspname || '.' || c.relname || '.' || a.attname
  from pg_attribute a
  join pg_class c on c.oid = a.attrelid
  join pg_namespace n on n.oid = c.relnamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway')
   and c.relkind in ('r', 'v', 'm', 'p')
   and a.attnum > 0 and not a.attisdropped
   and a.attname ilike '%project%'
union all
select 'type ' || n.nspname || '.' || t.typname
  from pg_type t join pg_namespace n on n.oid = t.typnamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway')
   and t.typname ilike '%project%'
union all
select 'constraint ' || c.conname
  from pg_constraint c join pg_namespace n on n.oid = c.connamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway')
   and c.conname ilike '%project%'
union all
select 'function ' || n.nspname || '.' || p.proname || '(' || pg_get_function_arguments(p.oid) || ') -> '
       || pg_get_function_result(p.oid)
  from pg_proc p join pg_namespace n on n.oid = p.pronamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway')
   and (p.proname ilike '%project%'
        or pg_get_function_arguments(p.oid) ilike '%project%'
        or pg_get_function_result(p.oid) ilike '%project%')
union all
select 'enum label ' || n.nspname || '.' || t.typname || ' ' || e.enumlabel
  from pg_enum e
  join pg_type t on t.oid = e.enumtypid
  join pg_namespace n on n.oid = t.typnamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway')
   and e.enumlabel ilike '%project%'
   -- A compiler's projects (rust-analyzer, tsc): another system's word.
   and not (t.typname = 'task_execution_kind' and e.enumlabel = 'compile_projects');

create temp view sensei_project_keys as
select 'metric key ' || key as what from sensei.metrics where key = 'project_health'
union all
select 'config key ' || key from sensei.config where key = 'setup.projects'
union all
select 'ladder rung ' || key from sensei.scopes where key = 'project'
union all
select 'namespace ' || scope_key || ':' || slug from sensei.namespaces where scope_key = 'project'
union all
select 'inbox artifact scope ' || id from sensei.dojo_inbox where scope ? 'project'
union all
select 'memory history scope ' || id from history.past_memories where scope = 'project';

do $$
declare
    left_over text;
begin
    select string_agg(what, E'\n  ' order by what) into left_over from sensei_project_names;
    if left_over is not null then
        raise exception E'the converged schema still says project:\n  %', left_over;
    end if;
    select string_agg(what, E'\n  ' order by what) into left_over from sensei_project_keys;
    if left_over is not null then
        raise exception E'the converged database still stores a project key:\n  %', left_over;
    end if;
    if not exists (select 1 from sensei.metrics where key = 'workspace_health') then
        raise exception 'the metric catalogue has no workspace_health';
    end if;
    if not exists (select 1 from sensei.scopes where key = 'workspace' and level = 60) then
        raise exception 'the ladder has no workspace rung at level 60';
    end if;
end $$;

-- ── a pre-rename shape, with a row in each renamed table ──────────────────────
alter table sensei.workspaces rename to projects;
alter table sensei.projects rename constraint workspaces_pkey to projects_pkey;
alter table sensei.repositories_in_workspaces rename to repositories_in_projects;
alter table sensei.repositories_in_projects rename column workspace_id to project_id;
alter table sensei.extension_workspaces rename to extension_projects;
alter table sensei.extension_projects rename column workspace_id to project_id;
alter table sensei.service_workspaces rename to service_projects;
alter table sensei.service_projects rename column workspace_id to project_id;
alter table activity.sessions rename column workspace_id to project_id;
alter table sensei.memories rename column workspace_id to project_id;
alter table sensei.mcp_servers rename column workspace_id to project_id;
alter table inference.corrections rename column workspace_ids to project_ids;
alter table sensei.workflow_state rename column workspace to project;
alter table staging.memories rename column workspace_name to project_name;
alter index activity.sessions_workspace_id_idx rename to sessions_project_id_idx;
alter table sensei.repository_metrics rename constraint repository_metrics_pkey to project_metrics_pkey;
alter type sensei.workspace_maturity rename to project_maturity;
alter type sensei.memory_scope rename value 'workspace' to 'project';
alter type sensei.entity_scope rename value 'workspace' to 'project';
alter type sensei.extension_scope rename value 'workspace' to 'project';
alter type sensei.sync_entity rename value 'workspace' to 'project';
alter type sensei.sync_entity rename value 'workspace_repository' to 'project_repository';
alter type sensei.task_execution_kind rename value 'analyze_workspace' to 'analyze_project';
alter type sensei.task_execution_kind rename value 'compute_workspace_metrics' to 'compute_project_metrics';
create function sensei.sole_project_of(p_folder_id uuid) returns uuid language sql stable as $$ select null::uuid $$;

-- The history trigger already reads the renamed column; an old database's
-- reads the old one. Off for the fixture, back on once the hook has run.
alter table sensei.memories disable trigger user;

insert into sensei.projects (id, name, maturity)
values ('e0000000-0000-4000-8000-0000000307a3', 'ztest-r3a', 'discovery');
insert into sensei.memories (project_id, scope, type, title, content)
values ('e0000000-0000-4000-8000-0000000307a3', 'project', 'convention', 'ztest-r3a', 'ztest-r3a');
insert into sensei.workflow_state (project) values ('ztest-r3a');
insert into inference.recommendations (workspace_id, title, why, action_type)
values ('e0000000-0000-4000-8000-0000000307a3', 'ztest-r3a', 'ztest-r3a', 'cross_project');

-- The stored keys as an older database holds them.
create temp table health_metric as select id from sensei.metrics where key = 'workspace_health';
update sensei.metrics set key = 'project_health' where key = 'workspace_health';
delete from sensei.config where key = 'setup.workspaces';
insert into sensei.config (key, value) values ('setup.projects', 'done');

-- The ladder as an older database holds it: the rung keyed `project`, every
-- workspace namespace on it, one with a stance attached (an FK on its id), and
-- an inbox artifact tagged with the old scope key.
insert into sensei.scopes (key, name, level, shareable, description, modified_at)
select 'project', 'Project', level, shareable, 'Rules for one logical project.', '2026-06-05'
  from sensei.scopes where key = 'workspace';
update sensei.namespaces set scope_key = 'project' where scope_key = 'workspace';
delete from sensei.scopes where key = 'workspace';
insert into sensei.namespaces (id, scope_key, name, slug)
values ('e0000000-0000-4000-8000-0000000307b1', 'project', 'ztest-r3b', 'ztest-r3b');
insert into sensei.stances (user_key, namespace_id)
values ('ztest-r3b', 'e0000000-0000-4000-8000-0000000307b1');
insert into sensei.dojo_memberships (id, registry_url, tenant_key, dojo_url, kind)
values ('e0000000-0000-4000-8000-0000000307b2', 'https://ztest', 'org/ztest', 'https://ztest/org/ztest', 'employer');
insert into sensei.dojo_inbox (membership_id, artifact_seq, artifact_signature, kind, title, body, scope)
values ('e0000000-0000-4000-8000-0000000307b2', 1, 'ztest-r3b', 'principle', 'ztest-r3b', 'ztest-r3b',
        '{"project": "ztest-r3b", "stack": "rust"}');
insert into history.past_memories (memory_id, scope, type, title, content, strength, status,
                                   operation, effective_from)
values (gen_random_uuid(), 'project', 'convention', 'ztest-r3b', 'ztest-r3b', 1, 'active',
        'UPDATE', now());

create temp table counts_before as
select (select count(*) from sensei.projects)                 as workspaces
     , (select count(*) from sensei.repositories_in_projects) as repositories_in_workspaces
     , (select count(*) from sensei.extension_projects)       as extension_workspaces
     , (select count(*) from sensei.service_projects)         as service_workspaces
     , (select count(*) from sensei.memories where scope = 'project') as workspace_memories
     , (select count(*) from activity.sessions where project_id is not null) as attributed_sessions
     , (select count(*) from sensei.namespaces where scope_key = 'project') as workspace_namespaces;

\ir ../../apply/before/sensei_workspaces_renamed.sql

alter table sensei.memories enable trigger user;

-- ── the rows came through under the new names ────────────────────────────────
do $$
declare
    before counts_before;
    left_over text;
begin
    select * into before from counts_before;

    if to_regclass('sensei.projects') is not null then
        raise exception 'sensei.projects is still there';
    end if;
    if (select count(*) from sensei.workspaces) <> before.workspaces
       or (select count(*) from sensei.repositories_in_workspaces) <> before.repositories_in_workspaces
       or (select count(*) from sensei.extension_workspaces) <> before.extension_workspaces
       or (select count(*) from sensei.service_workspaces) <> before.service_workspaces then
        raise exception 'a renamed table lost rows';
    end if;
    if (select count(*) from sensei.memories where scope = 'workspace') <> before.workspace_memories then
        raise exception 'memories scoped to a project did not become scoped to a workspace';
    end if;
    if (select count(*) from activity.sessions where workspace_id is not null) <> before.attributed_sessions then
        raise exception 'sessions lost their workspace';
    end if;
    if not exists (select 1 from sensei.workspaces w
                     join sensei.memories m on m.workspace_id = w.id
                    where w.id = 'e0000000-0000-4000-8000-0000000307a3'
                      and w.maturity = 'discovery'::sensei.workspace_maturity
                      and m.scope = 'workspace') then
        raise exception 'the fixture workspace or its memory did not survive the rename';
    end if;
    if not exists (select 1 from sensei.workflow_state where workspace = 'ztest-r3a') then
        raise exception 'workflow_state.project did not become workflow_state.workspace';
    end if;
    if exists (select 1 from inference.recommendations where action_type = 'cross_project')
       or not exists (select 1 from inference.recommendations
                       where workspace_id = 'e0000000-0000-4000-8000-0000000307a3'
                         and action_type = 'cross_workspace') then
        raise exception 'a cross_project recommendation did not become cross_workspace';
    end if;
    if not exists (select 1 from information_schema.columns
                        where table_schema = 'staging' and table_name = 'memories'
                          and column_name = 'workspace_name') then
        raise exception 'staging.memories.project_name was not renamed';
    end if;
    if not ('compute_workspace_metrics' = any (enum_range(null::sensei.task_execution_kind)::text[])
            and 'workspace_repository' = any (enum_range(null::sensei.sync_entity)::text[])
            and 'workspace' = any (enum_range(null::sensei.extension_scope)::text[])) then
        raise exception 'an enum label was not renamed';
    end if;
    if exists (select 1 from sensei.metrics where key = 'project_health')
       or not exists (select 1 from sensei.metrics m join health_metric h on h.id = m.id
                       where m.key = 'workspace_health') then
        raise exception 'project_health did not become workspace_health in place (its id kept)';
    end if;
    if exists (select 1 from sensei.config where key = 'setup.projects')
       or not exists (select 1 from sensei.config where key = 'setup.workspaces' and value = 'done') then
        raise exception 'the setup.projects config key did not become setup.workspaces with its value';
    end if;
    if exists (select 1 from sensei.scopes where key = 'project')
       or not exists (select 1 from sensei.scopes where key = 'workspace' and level = 60) then
        raise exception 'the ladder''s project rung did not become workspace';
    end if;
    if (select count(*) from sensei.namespaces where scope_key = 'workspace') <> before.workspace_namespaces then
        raise exception 'namespaces on the project rung did not all move to workspace';
    end if;
    if not exists (select 1 from sensei.stances s
                     join sensei.namespaces n on n.id = s.namespace_id
                    where s.user_key = 'ztest-r3b'
                      and n.id = 'e0000000-0000-4000-8000-0000000307b1'
                      and n.scope_key = 'workspace' and n.slug = 'ztest-r3b') then
        raise exception 'the fixture namespace (and the stance on it) is not found under workspace';
    end if;
    if not exists (select 1 from sensei.dojo_inbox
                    where artifact_signature = 'ztest-r3b'
                      and scope = '{"workspace": "ztest-r3b", "stack": "rust"}'::jsonb) then
        raise exception 'an inbox artifact''s project scope tag did not become workspace';
    end if;
    if exists (select 1 from history.past_memories where scope = 'project') then
        raise exception 'the memory history still says project';
    end if;
    if not exists (select 1 from pg_constraint where conname = 'repository_metrics_pkey') then
        raise exception 'repository_metrics kept its stale project_metrics_ constraint names';
    end if;

    select string_agg(what, E'\n  ' order by what) into left_over from sensei_project_names;
    if left_over is not null then
        raise exception E'the renamed schema still says project:\n  %', left_over;
    end if;
end $$;

-- ── a second run is a no-op ──────────────────────────────────────────────────
create temp table catalogue_after as
select count(*) as relations from pg_class c join pg_namespace n on n.oid = c.relnamespace
 where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway');

\ir ../../apply/before/sensei_workspaces_renamed.sql

do $$
begin
    if (select count(*) from pg_class c join pg_namespace n on n.oid = c.relnamespace
         where n.nspname in ('sensei', 'activity', 'inference', 'history', 'staging', 'export', 'gateway'))
       <> (select relations from catalogue_after) then
        raise exception 'a second run of the hook changed the schema';
    end if;
end $$;

rollback;
