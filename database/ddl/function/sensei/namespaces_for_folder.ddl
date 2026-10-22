set search_path to sensei, extensions;

-- The namespaces governing a folder, on the only two axes that exist:
-- its REPOSITORY, and the WORKSPACE(S) that repository belongs to.
--
-- Governance never attaches to a directory. A caller, though, almost always
-- starts from a cwd — so it holds a folder and needs the lift. This is that
-- lift, in one place, because five resolvers need it: rules, rule-pack rules,
-- checker refs, stances, and the scope→namespace lookup the save path fails
-- closed on. Five hand-written copies of the same joins is how four of them
-- end up agreeing and one does not.
--
-- THE TWO AXES ARE NOT INTERCHANGEABLE.
--
--   repository → `repository_namespaces`. Facts about the checkout: the
--   organization, and the repository's own namespace, where the packs of its
--   detected stacks are adopted (#318 R6). Keyed on the repository so two clones
--   cannot disagree.
--
--   workspace → `folder_workspaces` → the `workspace`-scope namespace of the same NAME. A workspace's governance identity. Resolved through the workspace rather
--   than through a per-folder binding, which is what let eight folders claim a
--   workspace their repository does not belong to.
--
-- A repository serving two workspaces yields BOTH workspaces' namespaces: a rule
-- constrains, so obeying the union can never be less safe than obeying one.
--
-- A folder with no repository and no workspace yields the EMPTY SET — not an
-- error, and not every namespace. Nothing is attached, so nothing is governed
-- by a namespace; the callers' always-on `general`/`user` clauses still apply
-- on their own.
--
-- The workspace side joins on NAME because that is the key `upsert_namespace`
-- writes with (`upsert_namespace('workspace', workspace_name, slug)`), and there is
-- no FK from a workspace to its namespace. Reading by the same key the writer
-- used is correct today; giving `sensei.workspaces` an explicit `namespace_id`
-- would make a rename safe, and is filed rather than smuggled in here.
create or replace function namespaces_for_folder(p_folder_id uuid)
returns setof uuid language sql stable as $$
  select rn.namespace_id
    from sensei.folders f
    join sensei.repository_namespaces rn on rn.repository_id = f.repository_id
   where f.id = p_folder_id
  union
  select n.id
    from sensei.folder_workspaces fp
    join sensei.workspaces p   on p.id = fp.workspace_id
    join sensei.namespaces n on n.scope_key = 'workspace' and n.name = p.name
   where fp.folder_id = p_folder_id
$$;

comment on function namespaces_for_folder(uuid) is
'Every namespace governing a folder, on both axes: its repository
(`repository_namespaces`) and the workspace(s) that repository belongs to
(`folder_workspaces` → the `workspace`-scope namespace of the same name). Empty
when neither is attached. Used by every governance resolver so the lift off a
folder is spelled exactly once.';
