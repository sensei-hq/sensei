-- Let `dojo.projects` become the shared project (#299) on a database that still
-- has the per-owner shape.
--
-- WHY THIS EXISTS. The reshape drops `projects.user_id`, and the old RLS
-- policies on the table reference it — so `dbd reconcile --allow-destructive`
-- fails with "cannot drop column user_id … because other objects depend on it"
-- before the policies step could replace them (it runs after reconcile). This
-- drops those policies first, ONLY while the old column is still there: once the
-- reshape has landed it is a no-op, so no later deploy ever runs with the read
-- policy removed. Until the policies step recreates it, the table has RLS on and
-- no read policy — it reads EMPTY, never open.
--
-- No-op where `dojo.projects` is absent (the daemon's own database).
do $$
begin
    if exists (select 1 from information_schema.columns
                where table_schema = 'dojo' and table_name = 'projects'
                  and column_name = 'user_id') then
        drop policy if exists projects_select     on dojo.projects;
        drop policy if exists projects_select_own on dojo.projects;
        drop policy if exists projects_file_own   on dojo.projects;
        drop policy if exists projects_refile_own on dojo.projects;
    end if;
end $$;
