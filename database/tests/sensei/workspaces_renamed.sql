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
-- WHAT BREAKS THIS TEST: a DDL file that still declares a `project` name; the
-- hook missing a table, column, type, enum label or constraint, so dbd would
-- recreate it empty or leave the old name behind; a rename that drops rows; a
-- hook that is not a no-op once it has run.
--
-- What keeps the word `project` is another system's: `mcp_servers.scope`
-- holds Claude Code's `'user' | 'project'` (a VALUE, not a name), the task
-- kind `compile_projects` compiles a compiler's projects, and the governance
-- ladder's `project` rung waits on the dōjō (see the hook).

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

do $$
declare
    left_over text;
begin
    select string_agg(what, E'\n  ' order by what) into left_over from sensei_project_names;
    if left_over is not null then
        raise exception E'the converged schema still says project:\n  %', left_over;
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

create temp table counts_before as
select (select count(*) from sensei.projects)                 as workspaces
     , (select count(*) from sensei.repositories_in_projects) as repositories_in_workspaces
     , (select count(*) from sensei.extension_projects)       as extension_workspaces
     , (select count(*) from sensei.service_projects)         as service_workspaces
     , (select count(*) from sensei.memories where scope = 'project') as workspace_memories
     , (select count(*) from activity.sessions where project_id is not null) as attributed_sessions;

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
