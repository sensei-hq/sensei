set search_path to sensei, extensions;

-- Where each node's definition lives, as a repo-relative path.
--
-- ## Why a view and not a column
--
-- R13 moved the path off `nodes` and onto `nodes.file_id -> files`. That is the
-- right shape — 16 bytes instead of a 47-byte string repeated on every node of
-- a file, and a foreign key that makes an untracked file impossible rather than
-- merely unlikely. What it costs is that every reader that wants a PATH now
-- needs two joins and the grain rule (`repo_relative_path`).
--
-- Roughly forty queries want exactly that. Written out at each one, the join is
-- forty chances to join to the wrong folder — and joining to the FILE's folder
-- instead of the NODE's silently yields a different path for every monorepo
-- member, which still looks like a path. Here it is decided once.
--
-- ## The anchor is the FILE's folder, not the node's
--
-- `sensei.files` is keyed `(folder_id, file_path)`, so the stored path is
-- relative to `files.folder_id` BY DEFINITION. That is the only folder it can
-- be prefixed with.
--
-- The node's folder is a different thing and the two can differ: a file belongs
-- to its deepest owner, while a node belongs to whichever folder the process
-- task carried. MEASURED while migrating — a node in `crates/member` whose file
-- resolved to the git root's row came back as
-- `crates/member/crates/member/src/lib.rs`, the member prefix applied twice.
--
-- The four views that each held a copy of this rule did not agree about it:
-- `doc_coverage` anchored on the file's folder, `symbols`, `graph_nodes` and
-- `file_tags` on the node's. Exactly the drift a single owner exists to end.
--
-- INNER joined on files: a node with no `file_id` is a reference stub or an
-- external `lib·` node, and it has no path to report. Callers that want those
-- rows too LEFT JOIN this.
drop view if exists node_paths;

create view node_paths as
select n.id        as node_id
     , n.folder_id
     , n.file_id
     , repo_relative_path(f.kind, f.path, fi.file_path) as file_path
  from nodes   n
  join files   fi on fi.id = n.file_id
  join folders f  on f.id  = fi.folder_id;

comment on view node_paths is
'Repo-relative path per node, the ONE place the files/folders join and the grain
rule live. Join it rather than re-deriving: `select ... from sensei.nodes n join
sensei.node_paths np on np.node_id = n.id`. Only nodes WITH a file appear —
reference stubs and external `lib·` nodes have none, so LEFT JOIN to keep them.';
comment on column node_paths.file_path is 'Repo-relative, via sensei.repo_relative_path().';
