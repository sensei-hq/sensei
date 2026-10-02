set search_path to sensei, extensions;

-- The project a folder belongs to, ONLY when there is exactly one.
--
-- The sole-member rule, in one place. `folder_projects` is multi-valued by
-- design — a repository is keyed on its REMOTE, so one serving two projects
-- resolves to both — and every consumer that needs a single answer applies the
-- same rule: take the member when there is one, NULL when there are two.
--
-- Written as a function because three call sites need it as a scalar inside a
-- larger query (`find_folder_for_path`, `get_folder_ids_by_path`, and session
-- attribution), and a fourth spelling of `case when count(*) = 1 then ...` is
-- how the rule starts to differ between them.
--
-- NULL means one of two things, and the caller must not conflate them:
--   - the folder belongs to no tracked repository, or its repository to no
--     project — there is no answer;
--   - the repository serves more than one project — there are two answers and
--     picking one would be a fabrication.
-- Read `sensei.folder_projects` directly when the difference matters.
--
-- STABLE, not IMMUTABLE: it reads tables, so the planner may cache it within a
-- statement but must not fold it at plan time.
create or replace function sole_project_of(p_folder_id uuid)
returns uuid
language sql
stable
as $$
  -- `(array_agg(...))[1]` rather than `min()`: Postgres has no `min(uuid)`,
  -- and this is the same spelling the migrated views use for the same rule.
  select case when count(*) = 1 then (array_agg(fp.project_id))[1] end
    from sensei.folder_projects fp
   where fp.folder_id = p_folder_id
$$;

comment on function sole_project_of(uuid) is
'The project a folder belongs to, ONLY when there is exactly one — the
sole-member rule that every consumer of the multi-valued `folder_projects`
applies, kept in one place so they cannot drift apart.

NULL is TWO different states and the caller must not conflate them: no
membership at all, or more than one. Read `sensei.folder_projects` when the
difference matters.';
