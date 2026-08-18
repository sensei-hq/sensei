set search_path to sensei, extensions;

-- The single, canonical repo-anchor resolver (spec docs/spec/2026-08-18-project-repo-session-model).
-- Maps ANY absolute path (a hook cwd, a transcript cwd, a watched file) to the REPO it
-- belongs to — the nearest ancestor folder that is a repo anchor — so sessions and
-- transcripts attach to a durable repo, never a transient subfolder. This is the ONE
-- implementation: PgStore::resolve_repo_anchor wraps it and every set-based reconcile calls
-- it, so per-event attribution and batch backfills can never diverge (the defect this
-- replaces: three resolvers — find_folder_for_path / repo_root_for_path /
-- get_folder_ids_by_path — that disagreed on kind, alias-awareness, and ancestor-walk).
--
-- Repo-anchor kinds (D1/D6): git + subtree (real repos), plus a standalone dir that is a
-- tracked project root (project_id set). workspace_member / sibling / folder are NOT anchors —
-- a monorepo package rolls up to its enclosing git root, a plain folder to its repo.
-- Alias-aware (move/rename): a path recorded before a move resolves via folder_path_aliases
-- to the current repo, so history is never lost. Resolution: the DEEPEST matching anchor
-- wins (a subtree inside a git root beats the outer root); a live abs_path beats an
-- equal-length alias. The prefix test is left()= rather than LIKE, so a repo name containing
-- '_' (e.g. next_js_app) can't act as a single-char wildcard.
drop function if exists repo_anchor_for(text) cascade;

create or replace function repo_anchor_for(p_path text)
returns table (
  repo_folder_id uuid,
  project_id     uuid,
  repo_abs_path  text,
  matched_via    text
)
language sql
stable
set search_path = sensei, extensions
as $$
  with anchors as (
    select f.id, f.project_id, f.abs_path as p, 1 as live
      from sensei.folders f
     where f.kind in ('git', 'subtree')
        or (f.kind = 'standalone' and f.project_id is not null)
    union all
    select f.id, f.project_id, a.alias_abs_path as p, 0 as live
      from sensei.folder_path_aliases a
      join sensei.folders f on f.id = a.folder_id
     where f.kind in ('git', 'subtree')
        or (f.kind = 'standalone' and f.project_id is not null)
  )
  select c.id, c.project_id, c.p, case when c.live = 1 then 'live' else 'alias' end
    from anchors c
   where p_path = c.p
      or left(p_path, length(c.p) + 1) = c.p || '/'
   order by length(c.p) desc, c.live desc
   limit 1;
$$;

comment on function repo_anchor_for is
'Canonical repo-anchor resolver (spec 2026-08-18): abs path -> owning repo
(repo_folder_id, project_id, repo_abs_path, matched_via live|alias). Nearest repo-kind
ancestor (git/subtree/standalone-project-root; monorepo members roll up to the git root),
alias-aware for move/rename, deepest+live wins, left()= prefix (underscore-safe). The ONE
mapper — Rust resolve_repo_anchor wraps it; all attribution/import/synthesis/repair/
reconcile call it.';
