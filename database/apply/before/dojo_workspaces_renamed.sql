-- Rename the dōjō's projects to workspaces IN PLACE (#307 D12, user 2026-10-21:
-- a project is a workspace — a group of repositories — everywhere).
--
-- WHY THIS EXISTS. The design renames dojo.projects, its links, viewer grants
-- and team links, their `project_id` columns and the two enums. dbd sees renamed
-- files as new entities, so on its own it would create empty `workspace` tables
-- and prune the old ones — losing every filed workspace, repository link and
-- viewer grant. This renames them first, so the reconcile finds them already in
-- place. It runs ONLY while `dojo.projects` still exists: afterwards it is a
-- no-op.
--
-- dojo.relay_sessions.project_id is NOT renamed here: dbd plans before this
-- hook runs, so it adds `workspace_id` and drops `project_id` itself (a rename
-- here would collide with its add). A run's workspace link is re-made by
-- dojo.attach_run_to_workspace on the run's next push; a finished run loses it
-- (pre-v1, data drops approved).
--
-- It also drops the dōjō policies that reference the functions this rename
-- retires (dojo.can_read_project & co., dropped by apply/after/
-- retired_functions.sql): a function a policy uses cannot be dropped. `dbd
-- policies` recreates them under the new names; until then the tables have RLS
-- on and no policy — they read EMPTY, never open.
--
-- No-op where dojo.projects is absent (the daemon's own database).
do $$
declare
    r record;
begin
    if to_regclass('dojo.projects') is null or to_regclass('dojo.workspaces') is not null then
        return;
    end if;

    for r in select tablename, policyname from pg_policies
              where schemaname = 'dojo'
                and (tablename in ('projects', 'project_viewers', 'team_projects', 'repositories_in_projects')
                     or qual ilike '%project%' or with_check ilike '%project%') loop
        execute format('drop policy if exists %I on dojo.%I', r.policyname, r.tablename);
    end loop;

    alter table dojo.projects                 rename to workspaces;
    alter table dojo.repositories_in_projects rename to repositories_in_workspaces;
    alter table dojo.project_viewers          rename to workspace_viewers;
    alter table dojo.team_projects            rename to team_workspaces;

    alter table dojo.repositories_in_workspaces rename column project_id to workspace_id;
    alter table dojo.workspace_viewers          rename column project_id to workspace_id;
    alter table dojo.team_workspaces            rename column project_id to workspace_id;

    -- A leftover workspace_* type that no column uses (a database that once held
    -- this rename and was converged back) would block the type rename: drop it.
    for r in select t.typname from pg_type t join pg_namespace n on n.oid = t.typnamespace
              where n.nspname = 'dojo' and t.typname in ('workspace_classification', 'workspace_phase')
                and not exists (select 1 from pg_attribute a where a.atttypid = t.oid and not a.attisdropped) loop
        execute format('drop type dojo.%I', r.typname);
    end loop;

    alter type dojo.project_classification rename to workspace_classification;
    alter type dojo.project_phase          rename to workspace_phase;

    -- Constraint and index names follow (a constraint's rename renames its index),
    -- on the four renamed tables only.
    for r in select c.conrelid::regclass as rel, c.conname
               from pg_constraint c
              where c.conrelid in ('dojo.workspaces'::regclass, 'dojo.repositories_in_workspaces'::regclass,
                                   'dojo.workspace_viewers'::regclass, 'dojo.team_workspaces'::regclass)
                and c.conname like '%project%' loop
        execute format('alter table %s rename constraint %I to %I', r.rel, r.conname,
                       replace(replace(r.conname, 'projects', 'workspaces'), 'project', 'workspace'));
    end loop;
    for r in select i.relname from pg_index x join pg_class i on i.oid = x.indexrelid
              where x.indrelid in ('dojo.workspaces'::regclass, 'dojo.repositories_in_workspaces'::regclass,
                                   'dojo.workspace_viewers'::regclass, 'dojo.team_workspaces'::regclass)
                and i.relname like '%project%' loop
        execute format('alter index dojo.%I rename to %I', r.relname,
                       replace(replace(r.relname, 'projects', 'workspaces'), 'project', 'workspace'));
    end loop;
end $$;
