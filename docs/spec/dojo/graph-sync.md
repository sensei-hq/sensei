# Daemon → dōjō graph sync — a repository's code graph, shared (#225)

> **Status:** plan. Q1 (§9) answered 2026-10-14: default branch only.
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
| `folders` | repository-relative `path`, `kind`, `role`, `name` | packages and modules; the world view groups by them |
| `schema_*` | as is, keyed by node | the schema view |
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
  hash differs from the hash the dōjō last accepted for that path. The accepted
  hashes come back with the plan (one map per repository), so the daemon needs
  no local watermark beyond `sync_state` status.
- **Replace per file.** The dōjō deletes the file's nodes and edges and inserts
  the slice, in one transaction per batch — the rule `call_sites` already
  follows locally (the fact belongs to the file that states it).
- **Deletion.** Each push for a repository carries its complete file manifest
  (`path → content_hash`); the dōjō prunes files absent from it. A manifest is
  small (5,302 paths here) and makes a rename or delete converge without a
  tombstone protocol.
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
- `dojo.graph_files`, `dojo.graph_folders`, `dojo.graph_nodes`,
  `dojo.graph_edges`, `dojo.graph_schema_*` — each with `tenant_id`,
  `repository_id` → `dojo.repositories(id)`, RLS on the same predicate as
  `repository_metrics` (a member of the tenant reads; nobody writes directly).
- `dojo.file_graph_slices(repo_key, manifest, slices)` — SECURITY DEFINER,
  the `file_metric_rows` contract: principal from the session, repository checked
  against `all_my_repositories`, whole batch refused on any failure, tenant from
  the verdict.
- `GET /v1/you/sync/plan` returns, per allowed repository, the accepted
  `path → content_hash` map.

## 5. The cycle

After the metrics push in `tick`, for each allowed repository with a graph:

1. Build the manifest from `files` (repository-relative paths).
2. Diff it against the plan's accepted hashes → changed and deleted paths.
3. Read the changed files' slices (§2), rewrite paths, batch to the cap.
4. `POST /v1/you/graph` per batch; the last batch of a repository carries the
   manifest so the dōjō prunes.
5. Record `sync_state(repository_graph, repo_key)` as `synced`, `skipped` (no
   graph yet) or `failed` with the reason; a refused oversized slice is counted
   in the reason.

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
these tables, scoped tenant → team → projects. The local state classes and
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

## 9. Decided: whose checkout (Q1)

Alice indexed `main` this morning; Bob is on a feature branch with half the
files changed. Both push the same repository. **Decided 2026-10-14: the dōjō
holds the default branch only; per file, the newest commit wins.** The team
sees `main`; feature-branch work stays local. Rejected: newest commit on any
branch (unmerged work leaks into the shared graph) and one graph per branch
(storage × branches, and a branch picker on every screen).

- **The daemon enforces the branch.** It pushes a repository's graph only while
  the clone is on the default branch, read from the remote
  (`git symbolic-ref refs/remotes/origin/HEAD`); otherwise the repository is
  `skipped` with reason `not_on_default_branch`. An unknown default branch is
  also skipped, never guessed. The dōjō cannot verify a branch name, so it does
  not pretend to.
- **The dōjō enforces the order.** Each slice carries its commit and the
  commit's time; `dojo.graph_files` keeps both, and a slice older than the stored
  one for that path is refused and counted — a member a few commits behind does
  not roll the shared graph back.
- **The manifest prunes only with a newer commit.** A manifest older than the
  repository's newest accepted commit prunes nothing.
