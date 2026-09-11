set search_path to sensei, extensions;

-- The ONE rule that turns a `files` row into a path a human can paste.
--
-- ## The grain problem it solves
--
-- R13 made `sensei.files` the file entity and `nodes.file_id` a key into it. A
-- `files` row is keyed `(folder_id, file_path)` and its path is relative TO ITS
-- OWN FOLDER — which for a monorepo member is the member directory, not the
-- repo root. So `crates/senseid/src/main.rs` is stored as `src/main.rs` under
-- the folder `crates/senseid`.
--
-- Every consumer wants the repo-relative form. Reconstructing it is one line —
-- and that one line was written out four times (symbols, graph_nodes,
-- doc_coverage, file_tags) before it had an owner. Four hand-copies of a rule
-- is how a rule drifts; measured on 18 folders, getting it wrong is SILENT,
-- because a folder-relative path is still a plausible-looking path.
--
-- `git`-kind folders ARE the repo root, so their files are already
-- repo-relative and prefixing would produce `sensei/src/...` for a repo called
-- sensei. Every other kind (module, subtree, standalone, folder) sits at
-- `folders.path` below the root and needs it.
--
-- IMMUTABLE and a single SELECT so Postgres inlines it: this appears in a
-- projection over hundreds of thousands of node rows, and a function-call
-- boundary per row would be the wrong price for one shared rule.
drop function if exists repo_relative_path(sensei.folder_kind, text, text) cascade;

create or replace function repo_relative_path(
  folder_kind sensei.folder_kind,
  folder_path text,
  file_path   text
)
returns text
language sql
immutable
set search_path = sensei, extensions
as $$
  select case
           when file_path is null then null
           when folder_kind = 'git' then file_path
           else folder_path || '/' || file_path
         end;
$$;

comment on function repo_relative_path is
'Repo-relative path for a files row: its folder-relative path prefixed with the
folder''s own path, except for git-kind folders which ARE the repo root. The ONE
owner of that rule — symbols, graph_nodes, doc_coverage and file_tags each held
a hand-copy before this existed. NULL in, NULL out.';
