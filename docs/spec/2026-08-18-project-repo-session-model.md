# Spec — Project / Repo / Folder / Session model (single-mapper redesign)

- **Status:** DRAFT for review — no code until the "Open decisions" are confirmed.
- **Date:** 2026-08-18
- **Owner:** Jerry
- **Motivation:** The metric-screen review surfaced phantom "today" sessions on stale projects (site-svelte). Root cause is not a display bug — it is the project/repo/folder/session data model: three divergent path resolvers, sessions anchored to non-repo folders behind a hard cascade FK, no multi-repo project grouping, and projects with no root of their own. This spec defines the target model, one shared mapper, the edge-case rules, a locked baseline, and atomic tests so we get it right once.

Related: `[[project_metrics_review_batch_2026_08_18]]`, `[[project_transcript_folder_rename_resilience]]`, `[[reference_sensei_user_primary_model]]`.

---

## 1. Principles (non-negotiable)

1. **A session/transcript belongs to a REPO, never to a transient folder.** Adding, deleting, or moving a *regular* folder (or a subfolder) must never change or lose a session/transcript's attribution.
2. **One shared mapper.** Exactly one resolver maps an absolute path → a repo anchor. Every call site (hook attribution, transcript import, cold-start synthesis, orphan repair, file watcher, scan) uses it. No second implementation.
3. **Repo delete = disable, not destroy.** A repo whose directory disappears is `archived`, not row-deleted. History (sessions, events, transcripts, metrics) is retained. Only true throwaway rows (regular subfolders of a vanished dir) are removed.
4. **Move/rename never loses history.** A repo that moves keeps its identity; old paths become aliases; sessions/transcripts recorded under the old path still resolve.
5. **Project = 1+ repos with a known root.** Default one repo → one project; a collection of repos under a shared root (e.g. `~/Work/Alert`) is one multi-repo project. Rollups are a VIEW over member repos, so combine/split is a re-grouping, never a re-computation of session numbers.
6. **Baseline is lockable.** Once the mapping is implemented and verified against the captured reference, the invariants in §8 are frozen — changes to them require an explicit ADR.

---

## 2. Current state (the baseline we must not regress)

Captured 2026-08-18 from the live `sensei` DB (see §7 for how this becomes golden fixtures).

### 2.1 Folder taxonomy (enum `folder_kind`)
`git` (repository) · `subtree` (nested git repo) · `workspace_member` (monorepo package) · `sibling` (non-git sibling of git folders) · `standalone` (non-git dir, no git siblings) · `folder` (plain subdirectory).
Counts: git 82 · workspace_member 124 · standalone 54 · folder 8 526 · subtree 0.

**Repo (anchor) kinds** = `git`, `subtree`, and standalone-project-roots (see Open decision D1). **Non-anchor** = `workspace_member` (rolls up to its enclosing git root), `sibling`, `folder`.

### 2.2 The three divergent resolvers (the defect)
| fn | file:line | walk | alias-aware | kinds considered | used by |
|---|---|---|---|---|---|
| `find_folder_for_path` | folders.rs:635 | nearest ancestor | ✅ | **any** (incl. `folder`) | hook attribution → `record_session_event` |
| `repo_root_for_path` | folders.rs:664 | nearest ancestor | ❌ | `git,standalone,subtree` (no `workspace_member`) | file watcher |
| `get_folder_ids_by_path` | folders.rs:683 | **exact only** | ✅ | any (exact) | transcript importer, synthesis, repair |

Because attribution uses "nearest ancestor of any kind," sessions get pinned to `folder`/`standalone` rows; because the importer uses "exact only," a subdir cwd fails to resolve. They disagree by construction.

### 2.3 The reference map (cwd → resolved kind)
131 distinct hook/transcript cwds resolve to: `folder` 69 · `standalone` 39 · `workspace_member` 29 · `git` 24 · **unresolved 15**. So **84 land on non-repo targets** and **15 on nothing**. Sessions today: 245 on `git`, 33 on `standalone`, 7 on `workspace_member`, **3 on a plain `folder`**.

### 2.4 The 15 unresolved cwds — bucketed (edge cases to handle)
- **Container dirs above a repo:** `/Users/Jerry`, `/Users/Jerry/Developer/jovy`, `/Users/Jerry/Developer/sensei-hq` — a real parent that holds repos but is not itself tracked.
- **Alias-only repos:** `/Users/Jerry/Developer/dbd-rs` (+ subdirs) — resolvable via `folder_path_aliases`, but the abs_path-prefix probe misses it (the exact bug of the un-unified mapper).
- **Untracked repos:** `/Users/Jerry/Work/sica`, `/Users/Jerry/Work/sig`.
- **Foreign / junk cwds:** `/home/user/project`, `/tmp/test` — CI/other-machine/test paths that must never mint a local project.

### 2.5 `path` is repo-relative, not root-relative
`folders.path` stores basename for a repo and repo-relative for subfolders; the column comment claims "relative to the watch root." 8 742/8 786 rows disagree with `root/path`. **`abs_path` is the only trustworthy path field.** Fix the doc + any consumer that assumes root-relative `path`. (No data migration; `abs_path` already correct.)

### 2.6 Multi-repo grouping only works inside a monorepo
`~/Work/Alert/` holds ~15 repos — `alert-platform` (monorepo, correctly grouped with its `workspace_member`s) plus `base-app-android/api/communities/crons/webapp`, `example-alert-site`, `documentation.wiki`, `astro-static`, `next_js_app`, `future`, `References`, `specifications` — **each its own separate project.** The grouper unites a git root with its members but never unites sibling repos under a shared collection root. Expected: one **"Alert"** project spanning them.

### 2.7 Session durability defect
`activity.sessions.folder_id uuid NOT NULL REFERENCES sensei.folders(id) ON DELETE CASCADE`. When a folder row is pruned (repo move with stale row, or subfolder churn), the session cascade-deletes; the events (bare-text `session_id`, no FK) survive → orphaned → repaired with `now()` (the phantom "today" sessions). Symptom already fixed in `2680ae2e` (repair backfills real timestamps); this spec removes the *cause*.

---

## 3. Target model

### 3.1 The single mapper — `resolve_repo_anchor`
One function, one SQL expression behind it, used everywhere.

```
resolve_repo_anchor(abs_path) -> Option<RepoAnchor>
RepoAnchor { repo_folder_id, project_id, repo_abs_path, matched_via: Live | Alias, confidence }
```

Algorithm (deterministic, alias-aware, repo-only):
1. Candidate set = current `folders.abs_path` (live) ∪ `folder_path_aliases.alias_abs_path` (alias), **restricted to repo-anchor kinds** (D1).
2. Keep candidates where `abs_path = $p OR $p LIKE abs_path || '/%'` (self or ancestor).
3. Order by `length(candidate_path) DESC` (deepest/most-specific repo wins — a `subtree` inside a `git` root wins over the outer root), then `live DESC` (a live path beats an equal-length alias).
4. `LIMIT 1`. `None` when nothing matches → caller applies the unresolved policy (§5, E5).

This replaces all three resolvers. `find_folder_for_path` (any-kind ancestor) is retired for attribution; the raw cwd folder is still recorded separately for provenance (§3.3) but is never the anchor. Exposed as **both** a Rust method (`PgStore::resolve_repo_anchor`) and a SQL `sensei.repo_anchor_for(text)` set-returning helper, so batch reconciles and per-event attribution share identical logic (a single tested SQL body; the Rust method wraps it).

### 3.2 Session → repo anchoring
- Add `activity.sessions.repo_folder_id uuid REFERENCES sensei.folders(id) ON DELETE SET NULL` — the durable anchor from the mapper. Recency, FTR, metrics, and rollups key off **this**, not the raw folder.
- Keep `folder_id` (make it nullable + `ON DELETE SET NULL`) purely as *raw cwd provenance* — "which exact dir the session ran in" — never used for attribution.
- `project_id` stays (derived from the anchor's project), `ON DELETE SET NULL`.
- Net effect: pruning any folder can never delete a session; at worst it nulls a pointer the next scan re-resolves.

> D2 (durability depth): `repo_folder_id` still points at a folder row. For maximum resilience we can *additionally* denormalize a **repo identity** (primary git remote URL, or `abs_path` when remote-less) onto the session, so attribution survives even a full folder-row rebuild. Recommended: yes, store `repo_key` (remote-or-path) alongside `repo_folder_id`.

### 3.3 Transcript → repo anchoring
Transcripts (Claude files under `~/.claude/projects/<encoded-cwd>`, Zed threads) carry a cwd. The importer/synthesis/repair all call `resolve_repo_anchor(cwd)` — same rule as sessions. A transcript recorded under an old path resolves via alias (move/rename safe). Synthesis writes `repo_folder_id` + `repo_key`; `set_session_history` continues to stamp real transcript timestamps.

### 3.4 Project = repo collection with a root
- Add `projects.root_abs_path text` (the collection root, e.g. `/Users/Jerry/Work/Alert`) — a project knows where it lives. (Multi-root projects, if ever needed, get a `project_roots` table; single `root_abs_path` for v1 — D3.)
- **Grouping algorithm** (`group_repos_into_projects`), default + collection:
  - Default: each repo-anchor folder → its own project (`root_abs_path = repo abs_path`).
  - Monorepo: a `git` root and its `workspace_member`s are one project (already works — preserve).
  - **Collection promotion (new):** sibling repos sharing a collection root are one multi-repo project. Trigger candidates (D4): (a) a shared parent dir that itself is a workspace marker (`~/Work/Alert/repos/*`, an Alert `.sensei`/workspace file), (b) a shared git-remote org/owner, (c) a shared name prefix (`base-app-*`) — **explicitly gated, conservative, and reversible via the UI**. Never auto-merge across watch roots.
- Combine/split is a re-assignment of `repo → project`, plus recompute of `projects.stack/root`; **no session rewrite** (sessions stay on their repo).

### 3.5 Rollup view (collapse/combine without reprocessing)
```
sensei.project_repo_sessions  -- session ⨝ repo_folder ⨝ project
sensei.project_observations   -- per-project rollup: session counts, FTR, metrics, recency
                              --   = UNION of member repos, grouped by project_id
```
Recency = `max(session.started_at)` over the project's member repos. A project's numbers are always a view over repos; splitting a project just changes which repos group under it — the view recomputes, session rows are untouched.

---

## 4. Schema changes (summary)
| change | table | note |
|---|---|---|
| `folder_id` → nullable, `ON DELETE SET NULL` | activity.sessions | manual `ALTER` (dbd won't alter existing table — see §9) |
| + `repo_folder_id uuid → folders ON DELETE SET NULL` | activity.sessions | the durable anchor |
| + `repo_key text` | activity.sessions | remote-or-path repo identity (D2) |
| + `root_abs_path text` | sensei.projects | project's collection root (D3) |
| view `project_repo_sessions`, `project_observations` | sensei.* | rollups (matview or plain view — D5) |
| SQL `sensei.repo_anchor_for(text)` | sensei.* | the shared mapper, set-based |
| fix column comment | sensei.folders.path | document repo-relative reality |

All additive except the `sessions.folder_id` FK change (needs a one-time migration + backfill of `repo_folder_id`/`repo_key` from existing events via the mapper).

---

## 5. Edge cases (detailed — each gets an atomic test in §6)

| id | scenario | required behavior |
|---|---|---|
| E1 | **Repo directory deleted on disk** | Folder row → `status='archived'` (NOT deleted). Sessions/events/transcripts/metrics retained. Excluded from active lists; still queryable in history. Reversible if the dir returns. |
| E2 | **Subfolder of a vanished dir** | The plain `folder` rows under a gone repo may be row-deleted (throwaway); MUST NOT touch any session (sessions never anchor to `folder` kind, so none is affected). |
| E3 | **Repo moved/renamed on disk** | Update the repo folder's `abs_path` to the new location; register the OLD abs_path as a `folder_path_aliases` row (`reason='detected'` if git-remote-matched, else `'rename'`). Sessions/transcripts under the old path resolve via alias → zero history loss. No cascade delete. |
| E4 | **Repo moved into a new intermediate dir** (site-svelte → `jovy/`) | Same as E3. The container dir (`jovy`) need not be tracked; `abs_path` is authoritative. The stale `/Developer/site-svelte` becomes an alias. |
| E5 | **Unresolved cwd** | Container dir above repos (`~/Developer/jovy`, `~/sensei-hq`): resolve to the deepest repo *under* it only for exact repo cwds; otherwise leave unattached (no phantom project). Foreign/junk (`/tmp/test`, `/home/user/*`, anything outside a watch root): **never mint a project or session** — drop with a logged skip. Untracked repo under a watch root (`~/Work/sica`): eligible for discovery on next scan, not fabricated. |
| E6 | **Monorepo member cwd** | A cwd inside a `workspace_member` anchors to the enclosing `git`/`subtree` repo root (members roll up), so all packages of one monorepo share the repo's sessions. |
| E7 | **Nested repo (`subtree`) inside a git root** | Deepest match wins: a cwd inside the subtree anchors to the subtree, not the outer git root. |
| E8 | **Standalone (non-git) project** (site-svelte) | D1 decides whether it is an anchor. Recommended: a standalone that is a tracked *project root* is a valid anchor; a standalone that is merely a sibling dir is not. |
| E9 | **Project combine** | Repos re-grouped under one project; `project_observations` view recomputes; session rows unchanged. |
| E10 | **Project split** | Repos re-assigned to separate projects; view recomputes; session rows unchanged (each session already on its repo). |
| E11 | **Two projects, same repo dir re-scanned** | Idempotent: the mapper + `abs_path UNIQUE` guarantee one repo row; re-scan never duplicates or re-orphans. |

---

## 6. Atomic tests (the acceptance gate)
Each is an isolated, DB-backed unit test with a fixed fixture; all must pass before the baseline is locked.

**Mapper (the spine) — `resolve_repo_anchor`:**
1. cwd == repo abs_path → that repo.
2. cwd deep inside a repo → that repo (nearest-ancestor).
3. cwd inside a `workspace_member` → enclosing git root (E6).
4. cwd inside a `subtree` within a git root → the subtree, not the outer root (E7).
5. cwd under a plain `folder` inside a repo → the repo, never the folder.
6. cwd == an aliased old path → the current repo (E3/E4).
7. foreign cwd (`/tmp/test`) → `None`, no side effect (E5).
8. container dir above repos → `None` (no phantom) (E5).
9. determinism: equal-length live vs alias → live wins; two aliases → stable order.

**Folder → project:** 10. one repo → one project (default). 11. monorepo git+members → one project. 12. collection root (`~/Work/Alert/repos/*`) → one multi-repo project (D4). 13. project carries correct `root_abs_path`.

**Transcript → repo:** 14. transcript cwd (subdir) → enclosing repo. 15. transcript under a renamed repo's old path → current repo (alias). 16. synthesized session carries real transcript timestamps + `repo_folder_id` (already guarded by the `2680ae2e` test — extend to assert the anchor).

**Session → repo:** 17. live hook event anchors session to the repo, not the cwd folder. 18. folder prune SET-NULLs `folder_id` but session + `repo_folder_id` + `project_id` survive (no cascade delete). 19. orphan repair re-anchors to the repo with historical timestamps.

**Views collapse/combine:** 20. `project_observations` FTR/recency == manual rollup over member repos. 21. combine two projects → view merges, session rows unchanged. 22. split → view separates, session rows unchanged. 23. archived repo (E1) still contributes to history rollups but is excluded from "active".

**Edge behaviors:** one test per E1–E11.

---

## 7. Baseline capture + lock-in
1. **Golden reference:** snapshot the current mapping to versioned fixtures under `docs/spec/baseline/2026-08-18/`:
   - `folder_refs.csv` — every distinct hook/transcript cwd → expected repo anchor (hand-verified).
   - `sessions_repo.csv` — every session → expected repo + project.
   - `projects.csv` — expected repo→project grouping incl. the Alert collection.
   These are the "correct answers" the new mapper must reproduce.
2. **Verification harness:** a test that runs `resolve_repo_anchor` over `folder_refs.csv` and asserts an exact match; a reconcile dry-run that asserts `sessions_repo.csv`/`projects.csv` are reproduced. Post-implementation this proves parity with the reviewed baseline.
3. **Lock:** once §6 is green and the harness matches the golden reference, tag the fixtures + the §8 invariants as frozen. A `LOCKED.md` in the baseline dir records the commit + date; changing a frozen invariant requires an ADR in `docs/decisions.md`.

---

## 8. Frozen invariants (once verified — no change without an ADR)
- I1. A session/transcript anchors to a **repo** (never a `folder`/`sibling`), via the single mapper.
- I2. Folder deletion/prune **cannot** delete a session (no cascade from folder → session).
- I3. Repo delete = `archived`; history retained (E1).
- I4. Move/rename preserves attribution via alias; no history loss (E3/E4).
- I5. `abs_path` is the authoritative path; `path` is repo-relative display only.
- I6. Exactly one mapper implementation; every call site uses it.
- I7. Project numbers are a **view** over member repos; combine/split never rewrites sessions.

---

## 9. Rollout phases
1. **P0 — Mapper (no schema):** implement `resolve_repo_anchor` (Rust + SQL) + tests 1–9; repoint attribution/import/synthesis/repair/watcher to it. Ship behind parity with the golden reference. (No FK change yet — `repo_folder_id` derived on read.)
2. **P1 — Session durability:** `sessions.folder_id → SET NULL` + add `repo_folder_id`/`repo_key`; backfill from events via the mapper; tests 17–19. Manual `ALTER` migration (dbd `create table if not exists` won't alter — apply via `psql`, mirror in DDL source, per `[[reference_dbd_reconcile_incremental]]`).
3. **P2 — Archive-not-delete + move/rename hardening:** reconcile archives vanished repos, prunes only throwaway `folder` rows, and registers aliases on move; tests E1–E7.
4. **P3 — Project roots + collection grouping:** `projects.root_abs_path` + `group_repos_into_projects` collection promotion (Alert case); tests 10–13.
5. **P4 — Rollup views:** `project_repo_sessions`, `project_observations`; repoint recency/metrics/observatory to them; tests 20–23. Retire ad-hoc per-folder recency.
6. **P5 — Lock:** run the verification harness against the golden reference; freeze §8.

Each phase: zero-errors gate + its atomic tests before merge.

---

## 10. Resolved decisions (confirmed 2026-08-18)
- **D1 — Standalone project root as anchor? → YES.** A tracked project-root standalone (site-svelte) is a valid repo-equivalent anchor; a mere sibling/child standalone is not. Predicate: `kind IN ('git','subtree') OR (kind='standalone' AND project_id IS NOT NULL)`.
- **D2 — `repo_key` on sessions? → YES.** Store the primary git remote (or `abs_path` when remote-less) so attribution survives a full folder-row rebuild.
- **D3 — Project root storage? → single `projects.root_abs_path`** now; `project_roots` table only if multi-root is ever needed.
- **D4 — Collection-promotion trigger? → shared collection root dir + shared git-remote org**, conservative and UI-reversible; never name-prefix alone, never across watch roots.
- **D5 — Rollup as → plain view** first; matview only if recency queries get hot.
- **D6 — Monorepo member cwd anchors to → the enclosing git root** (members roll up); a monorepo is one repo for session purposes.

## 11. Algorithm validation (2026-08-18, pre-implementation)
The §3.1 mapper was prototyped in SQL and run over every distinct hook/transcript cwd in the live DB with the confirmed decisions. Result: **169/176 cwds resolve** (133 live + 36 alias); the **7 unresolved are exactly the correct buckets** (container dirs `~/`, `~/Developer/jovy`, `~/Developer/sensei-hq`; untracked `~/Work/sica`, `~/Work/sig`; foreign `/home/user/project`, `/tmp/test`). Spot-checks pass: site-svelte→standalone anchor (D1); `dbd-rs/site/static`→`dbd-rs` git via **alias** (the case all three legacy resolvers missed); `alert-platform/apps/admin`→monorepo **git root** (D6); containers + foreign→unresolved. The algorithm is proven against real data; P0 implements it in Rust+SQL to reproduce this exact map (the §7 golden reference).
