# Daemon → dōjō graph sync — a repository's code graph, shared (#225)

> **Status:** plan. Q1 (§9) answered 2026-10-14: a configured SET of tracked
> branches per repository (the forge default, plus e.g. production), one graph
> each; each person contributes the tracked branches they have checked out.
>
> **Decided 2026-10-14:** sync the **raw graph** — nodes and edges — rather than
> derived diagram payloads or aggregates. A repository is shared, so its graph
> belongs to the whole repository, not to one person; every diagram view then
> works on the dōjō as it does locally. The confidentiality boundary is the one
> that already exists: a repository syncs only when it is elected to the tenant
> (`dojo.all_my_repositories.sync_enabled`). There is no separate graph switch on
> `tenant_share_policy`.

## 1. What exists

**Locally** every graph table hangs off `folder_id`; a folder reaches its
repository through `folders.repository_id`. Measured 2026-10-14 on the
development machine (2 indexed repositories):

| table | rows | size | read by the diagrams |
|---|---|---|---|
| `nodes` | 46,171 | 244 MB (heap 69 MB) | yes — `structure_graph`, `graph_nodes`, zones, world |
| `edges` | 209,142 | 400 MB (heap 171 MB; `props` 74 MB) | yes — `structure_edges`, neighbourhood, zones |
| `files` | 5,302 | 277 MB | yes — through the views |
| `folders` | 956 | 22 MB | yes — `folder_projects`, world |
| `schema_entities` / `_columns` / `_indexes` / `_refs` | 296 / 1,839 / 335 / 192 | 2 MB | yes — schema view |
| `call_sites` | 5,581 | 2.5 MB | no — input to `DeriveOrmSchema` only |
| `symbol_names` | 218,211 | 746 MB | no — drift scan; global, not per repository |

Per repository: up to 31,762 nodes and 140,645 edges; 44,259 edges unresolved
(`target_id` null). Every edge carries `props.occurrences`, keyed by file. The
gap analysis measured 745k nodes on this machine earlier, so the design must not
assume today's small corpus.

**The sync path** (`tasks/dojo_sync.rs`) already does the hard parts for
metrics, and graph sync reuses each of them:

- **Election** — the dōjō decides; the daemon pushes only `plan.allowed`.
- **Authorization** — the Worker resolves each `repo_key` through
  `my_repositories`, and the SECURITY DEFINER function checks every row's
  repository against `all_my_repositories` (`principal = session`,
  `sync_enabled`), refusing the whole batch otherwise; `tenant_id` comes from
  that verdict, never from the input (`file_metric_rows.ddl`).
- **Status** — `sensei.sync_state` per `(entity, entity_key)`.
- **Transport** — `user_plane.rs`, bodies capped at 1,000 rows / 2 MB.

**On the dōjō** there is no graph substrate. `dojo.repositories` is one row per
`(tenant_id, repo_key)`; the `dojo` scope deploys every table in the `dojo`
schema and has no `pgvector`.

## 2. What travels, and what does not

| local | travels as | why |
|---|---|---|
| `files` | `file_path` (repository-relative), `content_hash` | the unit of sync (§3) and its change marker |
| `nodes` | every column the views read: `kind`, `name`, `fqn`, `parent` (by key), `line_start/end`, `is_exported`, `is_test`, `signature`, `description`, `docstring`, `language`, `community_id`, `tags`, `props` | the diagrams read these |
| `edges` | `kind`, source (by key), target (by key, or `target_name` when unresolved), `confidence`, `resolved_via`, `unresolved_reason`, `props.occurrences` | the diagrams read these; unresolved edges are part of the graph |
| `schema_*` | on the node they describe, inside its slice; a ref's target as a node key | the schema view |
| `folders` | **does not travel** | measured 2026-10-14: the diagram views join `folders` only to reach a node's checkout for workspace membership — on the dōjō that is `tenant → team → projects → repositories`, which already exists. Every node, edge and file hangs off the checkout folder itself |
| `nodes.embedding` | **does not travel** | no `pgvector` on the dōjō; search there is a later decision |
| `nodes.content` | **does not travel** | empty today; source text is not graph |
| `folders.abs_path`, `remote_urls`, `branch` | **do not travel** | machine-local; the repository is already identified by `repo_key` |
| `call_sites`, `symbol_names` | **do not travel** | not rendered; derivation input and drift index |

**Paths become repository-relative before they leave the machine.** A local
`folders.path` / `abs_path` names the developer's disk; `files.file_path` is
folder-relative. The daemon rewrites both to paths relative to the repository
root, so every member's copy of a file has the same name on the dōjō.

## 3. The unit of sync is a file

A repository's graph is too large for one request (140k edges, ~74 MB of edge
`props`) and changes a few files at a time. So the unit is **a file's slice**:
the file row, the nodes declared in it, the edges whose source node is in it,
and the schema rows of its nodes.

- **Change detection** is `files.content_hash`. The daemon pushes a file when its
  hash differs from the hash the dōjō last accepted for that path, read with
  `GET /v1/you/graph?repo_key=&branch=` (`dojo.graph_file_hashes`) — gated like
  the write, so a syncing member on no team still sees it. Not in the plan: a
  plan carrying thousands of paths per tracked branch would be fetched every
  cycle for repositories with nothing to push. The daemon needs no local
  watermark beyond `sync_state` status.
- **Replace per file.** The dōjō deletes the file's nodes and edges and inserts
  the slice, in one transaction per batch — the rule `call_sites` already
  follows locally (the fact belongs to the file that states it).
- **Deletion.** Each push for a repository carries its complete file manifest
  (`path → content_hash`); the dōjō prunes files absent from it. A manifest is
  small (5,302 paths here) and makes a rename or delete converge without a
  tombstone protocol.
- **Nodes with no file** (6,032 here: 5,161 `unknown` placeholders, 583
  `package`, 187 `module`, 101 `field`; 45 edges leave them) travel as the
  repository's *unplaced* slice — a slice with a null path — carried with the
  manifest and replaced whole. A manifest never prunes them: they are not a file.
- **Batches never split a file.** A batch is whole slices up to the byte cap; a
  single slice over the cap is refused and counted, never truncated.

**Identity across machines.** Local uuids mean nothing on the dōjō. A node is
keyed `(repository_id, folder_path, fqn)` — `fqn` is unique per folder locally —
and an edge refers to its endpoints by those keys. The dōjō resolves a target key
to its own node row at read time, so an edge into a file not yet synced is kept
and resolves when that file arrives.

## 4. Schema changes

- `sensei.sync_entity` gains `repository_graph` (status per repository, keyed by
  `repo_key`). Shared-closure enum: it reaches both planes, and the `dojo`
  scope's `includes` must list it.
- `dojo.graph_files`, `dojo.graph_nodes`, `dojo.graph_edges`,
  `dojo.graph_schema_*` (hanging off `graph_nodes`, so a slice's replace takes
  them with its nodes) — each with `tenant_id`,
  `repository_id` → `dojo.repositories(id)`, RLS on the same predicate as
  `repository_metrics` (a member of the tenant reads; nobody writes directly).
- `dojo.graph_heads` — one row per (repository, branch): the newest accepted
  commit (`sha`, `committed_at`); the order rule for the unplaced slice and for
  manifests. Every `dojo.graph_*` row carries its branch.
- `dojo.repositories.default_branch` and `dojo.repository_graph_branches` (§9).
- `dojo.file_graph_slices(repository_id, commit, slices, manifest)` — SECURITY DEFINER,
  the `file_metric_rows` contract: principal from the session, repository checked
  against `all_my_repositories`, whole batch refused on any failure, tenant from
  the verdict.
- `GET /v1/you/sync/plan` returns, per allowed repository, its tracked branches.
- `POST /v1/you/graph` (a batch) and `GET /v1/you/graph` (the accepted hashes of
  one branch, paged).

## 5. The cycle

After the metrics push in `tick`, for each allowed repository with tracked
branches, each local checkout (a `git` folder) on one of them — one per branch:

1. **Is the index this branch's?** The live `HEAD` branch must be the branch
   the folder was indexed on; otherwise `skipped: index_not_current`. A
   checkout on an untracked branch is `skipped: not_a_tracked_branch`.
2. **Only committed work.** The local graph is the working tree's, so every
   path `git status --porcelain` reports (edited, added, deleted, untracked,
   both sides of a rename) is held back: not pushed, and kept in the manifest at
   the hash the dōjō already has, so it is not pruned either.
3. Diff the committed files' hashes against `GET /v1/you/graph` → changed paths.
4. Read their slices (§2) 200 files at a time, batch whole slices under 3 MB
   (the route caps at 4 MB), `POST /v1/you/graph` per batch.
5. Last request: the unplaced slice and the manifest, so the dōjō prunes.
6. Record `sync_state(repository_graph, <repo_key>@<branch>)`: `synced`, the
   `skipped` reason, or `error` — including a file whose single slice is over the
   cap, which is named rather than split.

A checkout's files are already repository-relative (`files.file_path` is
relative to the `git` folder), and every node and edge hangs off that folder,
so `folder_path` is `''`. An edge into another folder (another repository's
checkout) is sent with no target key, only its name.

A first sync is a few dozen requests at today's caps (edge `props` alone are
74 MB across the two repositories measured, at 2 MB a request); afterwards a
push is the files that changed.

## 6. Order of work

1. This spec; Q1 answered.
2. DDL: the `dojo.graph_*` tables, the enum value, RLS; `dbd inspect`; deploy
   twice (empty and populated).
3. `file_graph_slices` + a `database/tests/dojo/` test: a member of another
   tenant, an unelected repository and a non-member are each refused; a slice
   replaces exactly its file; a manifest prunes exactly the absent files.
4. Daemon: manifest, diff, slice reader, path rewriting, batching (unit-tested
   against a fixture graph, mutation-probed on the path rewrite — a leaked
   absolute path must fail a test).
5. Worker route `POST /v1/you/graph` + the plan's hash map.
6. Live: sync this repository to a test tenant; compare dōjō row counts per
   table with the local counts for the same repository (a conservation check),
   then change one file and confirm exactly its slice moves.

The dōjō's diagram screens are the next issue: they need dōjō loaders over
these tables, scoped tenant → team → workspaces. The local state classes and
components are reusable; the loaders are not.

## 7. Done gate

- After a first sync, every `dojo.graph_*` table holds, for the repository, the
  same row count as the local tables (minus what §2 excludes) — checked per table.
- No path on the dōjō starts with `/` or contains the developer's home
  directory, for any repository.
- Editing one file and re-syncing changes rows of that file only.
- Deleting a file locally removes its rows on the dōjō after the next sync.
- A member of another tenant, and a member for an unelected repository, read
  nothing and write nothing.

## 8. Not in scope

- The dōjō diagram screens (follow-up issue).
- Embeddings and graph search on the dōjō.
- Author attribution on graph rows: a graph is the repository's, so rows carry
  no person (anonymisation, #224, does not arise).

## 9. Decided: whose checkout, which branches (Q1)

Alice indexed `main` this morning; Bob is on a feature branch with half the
files changed. Both push the same repository. **Decided 2026-10-14: the dōjō
holds one graph per TRACKED branch — a small set configured per repository —
and never a mix of checkouts.** Unmerged work on an untracked branch stays local.

**The indexer already supports this.** A checkout is a folder with its own
graph; `main` and `production` checked out side by side (two clones, or git
worktrees) are two folders of one repository, each with its branch in
`folders.branch` (the column's comment: "develop vs main = two folders, one
repository"). There is no branch dimension INSIDE one folder, and none is needed.

**The set is the dōjō's setting.**

- `dojo.repositories.default_branch` — captured from the forge with visibility
  (GitHub's `/repos/{owner}/{repo}` returns `default_branch`; same read, same
  rule: a read that does not succeed writes nothing).
- `dojo.repository_graph_branches(repository_id, branch, label)` — the tracked
  branches beyond the default, e.g. `production`, `develop`. Added and removed
  by whoever holds authority over the repository's settings
  (`configurable_by_me`, the rule elections use).
- The tracked set is the default branch plus those rows. No default captured and
  no rows → the graph does not sync (`branch_unknown`), never a guessed `main`.
- Removing a branch from the set deletes its graph; adding one starts it empty.
  Nothing is dropped by a change elsewhere in the set.

**The dōjō enforces it.** Every slice and manifest carries its branch and commit;
`file_graph_slices` refuses a branch outside the set (`branch_not_tracked`). Per
branch and file the newest commit wins: an older slice is refused and counted, so
a member a few commits behind does not roll that branch back. A manifest older
than its branch's newest accepted commit prunes nothing.

**Each person contributes the tracked branches they have.** The plan lists each
repository's tracked set; the daemon pushes every local checkout whose branch is
in it — a person with `main` and `production` worktrees contributes both. A
checkout on any other branch is not pushed, and the app says so per checkout
(`not_a_tracked_branch`). Local diagrams always show the working tree.

**Storage is one graph per tracked branch** — which is why it is a configured
set and not every branch. Dōjō screens show a branch picker over the tracked set.
