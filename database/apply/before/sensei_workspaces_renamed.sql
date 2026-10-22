-- Rename sensei's projects to workspaces IN PLACE (#307 D12/D30, user
-- 2026-10-21: a project is a workspace — a group of repositories — everywhere,
-- the daemon's own database included).
--
-- WHY THIS EXISTS. The design renames sensei.projects, its three link tables,
-- every `project_id` column, the `project_maturity` type and the enum labels
-- that say project (D16: stored values convert too). dbd sees renamed files as
-- new entities, so on its own it would create empty `workspace` tables, drop the
-- `project_id` columns with their values and recreate the enums without the
-- rows' labels — losing every workspace, its repositories, every memory's and
-- session's attribution. This renames them first, so the reconcile finds them
-- already in place.
--
-- WHEN IT RUNS. The daemon's deploy (`sensei_bootstrap::database::deploy_to`)
-- runs `apply` — which runs this hook — and only then plans the reconcile, so
-- the reconcile sees the renamed schema. A bare `dbd reconcile` plans BEFORE its
-- hooks run and would collide with a column renamed here: on a database that
-- still has sensei.projects, converge with `dbd apply` first, then reconcile.
--
-- The table half runs ONLY while `sensei.projects` exists: afterwards, and on a
-- fresh database, it is a no-op. The enum-label half runs wherever a label is
-- still old, on any plane: `sensei.entity_scope` is also deployed to the dōjō.
-- So does the stored-key half (R3b): the retired `project_health` metric (the
-- catalogue is on both planes) and the setup wizard's `setup.projects` key.
--
-- It drops the views and functions whose names, columns or parameters say
-- project (a view cannot be replaced with a renamed column); `apply` recreates
-- each at the design's definition. A retired function is also dropped by
-- apply/after/retired_functions.sql.
--
-- NOT HERE: the governance ladder's `project` rung (sensei.scopes,
-- sensei.namespaces.scope_key). The dōjō shares that vocabulary over its wire
-- and in its own database, so it converts with the dōjō, not before it. Nor
-- `task_execution_kind`'s `compile_projects`: it compiles a COMPILER's projects
-- (rust-analyzer, tsc), another system's word.
do $$
declare
    r record;
    pair text[];
    item text;
    dropped text[];
    daemon_schemas constant text[] := array['sensei', 'activity', 'inference', 'history',
                                            'staging', 'export', 'gateway'];
begin
    -- ── enum labels (D16), on any plane ──────────────────────────────────────
    foreach pair slice 1 in array array[
            ['entity_scope',        'project',                 'workspace'],
            ['memory_scope',        'project',                 'workspace'],
            ['extension_scope',     'project',                 'workspace'],
            ['sync_entity',         'project',                 'workspace'],
            ['sync_entity',         'project_repository',      'workspace_repository'],
            ['task_execution_kind', 'analyze_project',         'analyze_workspace'],
            ['task_execution_kind', 'compute_project_metrics', 'compute_workspace_metrics']] loop
        if exists (select 1 from pg_enum e join pg_type t on t.oid = e.enumtypid
                     join pg_namespace n on n.oid = t.typnamespace
                    where n.nspname = 'sensei' and t.typname = pair[1] and e.enumlabel = pair[2])
           and not exists (select 1 from pg_enum e join pg_type t on t.oid = e.enumtypid
                             join pg_namespace n on n.oid = t.typnamespace
                            where n.nspname = 'sensei' and t.typname = pair[1] and e.enumlabel = pair[3]) then
            execute format('alter type sensei.%I rename value %L to %L', pair[1], pair[2], pair[3]);
        end if;
    end loop;

    -- ── stored keys (D16, R3b), on any plane ─────────────────────────────────
    -- Renamed in place, never re-inserted: the metric's id carries its recorded
    -- values, and the config row its value. Skipped when the new key is already
    -- there (a fresh database the import seeded).
    if to_regclass('sensei.metrics') is not null then
        update sensei.metrics set key = 'workspace_health'
         where key = 'project_health'
           and not exists (select 1 from sensei.metrics where key = 'workspace_health');
    end if;
    if to_regclass('sensei.config') is not null then
        update sensei.config set key = 'setup.workspaces'
         where key = 'setup.projects'
           and not exists (select 1 from sensei.config where key = 'setup.workspaces');
    end if;

    if to_regclass('sensei.projects') is null or to_regclass('sensei.workspaces') is not null then
        return;
    end if;

    -- ── what cannot be replaced in place: dropped, `apply` recreates it ───────
    -- Listed first, dropped after: a cascade can take a later one with it.
    select array_agg(format('%I.%I', n.nspname, c.relname)) into dropped
      from pg_class c join pg_namespace n on n.oid = c.relnamespace
     where c.relkind = 'v'
       and n.nspname = any(daemon_schemas)
       and (c.relname ilike '%project%'
            or exists (select 1 from pg_attribute a
                        where a.attrelid = c.oid and a.attnum > 0 and a.attname ilike '%project%')
            or pg_get_viewdef(c.oid) ilike '%project%');
    foreach item in array coalesce(dropped, '{}') loop
        execute format('drop view if exists %s cascade', item);
    end loop;
    select array_agg(format('%I.%I(%s)', n.nspname, p.proname,
                            pg_get_function_identity_arguments(p.oid))) into dropped
      from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = any(daemon_schemas)
       and p.prorettype <> 'trigger'::regtype
       and (p.proname ilike '%project%'
            or pg_get_function_arguments(p.oid) ilike '%project%'
            or pg_get_function_result(p.oid) ilike '%project%');
    foreach item in array coalesce(dropped, '{}') loop
        execute format('drop function if exists %s cascade', item);
    end loop;

    -- ── tables ───────────────────────────────────────────────────────────────
    alter table sensei.projects rename to workspaces;
    foreach pair slice 1 in array array[['repositories_in_projects', 'repositories_in_workspaces'],
                                        ['service_projects',         'service_workspaces'],
                                        ['extension_projects',       'extension_workspaces']] loop
        if to_regclass('sensei.' || pair[1]) is not null then
            execute format('alter table sensei.%I rename to %I', pair[1], pair[2]);
        end if;
    end loop;

    -- ── columns: every one a sensei-owned table names after a project ───────
    for r in select n.nspname, c.relname, a.attname
               from pg_attribute a
               join pg_class c on c.oid = a.attrelid
               join pg_namespace n on n.oid = c.relnamespace
              where c.relkind in ('r', 'p')
                and n.nspname = any(daemon_schemas)
                and a.attnum > 0 and not a.attisdropped
                and a.attname in ('project_id', 'project_ids', 'project', 'project_name') loop
        execute format('alter table %I.%I rename column %I to %I', r.nspname, r.relname, r.attname,
                       replace(r.attname, 'project', 'workspace'));
    end loop;

    -- ── stored words (D16): a recommendation's action verb is free text ──────
    update inference.recommendations set action_type = 'cross_workspace'
     where action_type = 'cross_project';

    -- ── the maturity type ────────────────────────────────────────────────────
    -- A leftover workspace_maturity no column uses (a database that once held
    -- this rename and was converged back) would block the rename: drop it.
    if exists (select 1 from pg_type t join pg_namespace n on n.oid = t.typnamespace
                where n.nspname = 'sensei' and t.typname = 'workspace_maturity'
                  and not exists (select 1 from pg_attribute a
                                   where a.atttypid = t.oid and not a.attisdropped)) then
        drop type sensei.workspace_maturity;
    end if;
    if to_regtype('sensei.project_maturity') is not null then
        alter type sensei.project_maturity rename to workspace_maturity;
    end if;

    -- ── constraint and index names follow ────────────────────────────────────
    -- repository_metrics kept the names of the table it was renamed from
    -- (`project_metrics`); a fresh database names them after repository_metrics.
    for r in select c.conname from pg_constraint c
              where c.conrelid = 'sensei.repository_metrics'::regclass
                and c.conname like 'project\_metrics\_%' loop
        execute format('alter table sensei.repository_metrics rename constraint %I to %I',
                       r.conname, 'repository_metrics_' || substr(r.conname, length('project_metrics_') + 1));
    end loop;
    -- A constraint's rename renames its index.
    for r in select c.conrelid::regclass as rel, c.conname
               from pg_constraint c join pg_namespace n on n.oid = c.connamespace
              where n.nspname = any(daemon_schemas)
                and c.conname like '%project%' loop
        execute format('alter table %s rename constraint %I to %I', r.rel, r.conname,
                       replace(replace(r.conname, 'projects', 'workspaces'), 'project', 'workspace'));
    end loop;
    for r in select n.nspname, i.relname
               from pg_class i join pg_namespace n on n.oid = i.relnamespace
              where i.relkind = 'i'
                and n.nspname = any(daemon_schemas)
                and i.relname like '%project%' loop
        execute format('alter index %I.%I rename to %I', r.nspname, r.relname,
                       replace(replace(r.relname, 'projects', 'workspaces'), 'project', 'workspace'));
    end loop;
end $$;
