# Plan — transcript/project resilience to repo folder rename & delete

> Make transcript→project mappings survive a repo folder rename or delete, correct
> the already-impacted repos (dbd-rs→dbd, strategos/monorepo→torii, strategos/
> gateway→gateway), and turn a "transcripts but no repo" case into an archived
> project instead of silent data loss. Daemon (Rust) feature. Plan-first because it
> touches project **identity** (the #109 area) and mutates the live `sensei` DB.

## Problem (confirmed against code + live DB)

Project/folder identity is **path-keyed**: the load-bearing key is
`sensei.folders.abs_path` (`not null unique`, `database/ddl/table/sensei/folders.ddl:12`).
Every transcript→project resolution is an **exact `abs_path` string match**:
- transcript synthesis → `get_folder_ids_by_path(cwd)` = `WHERE abs_path = $1`
  (`pg_store.rs:5324`, from `transcript/mod.rs:219`); no match → session skipped
  (`transcript/mod.rs:224`).
- live hook capture → `find_folder_for_path(cwd)` = nearest-ancestor `abs_path`
  (`pg_store.rs:4167`).

Git remote (`folders.remote_urls`) exists but is **never written in prod and never
consulted for identity**. Project rows have only `id` + `name`; a folder gets its
project by **basename** (`get_or_create_project_by_name`, `pg_store.rs:5529`).

**So on a rename today** (`scan.rs` reconcile): the old `abs_path` fails
`p.exists()` → `classify_stale_root` = `Remove` (`scan_logic.rs:637`) →
`delete_folder_tree` (`pg_store.rs:2057`) **hard-deletes the folder and
cascade-deletes its `activity.sessions`** (`sessions.ddl:4` FK). The renamed repo
is re-discovered as a NEW folder (new uuid) + NEW project (new basename).
`transcript_turns`/`transcript_cursor`/`assistant_events` (session-id-string keyed,
no FK) are left orphaned. Nothing bridges old↔new.

**Live state now** (`sensei` DB): `dbd` (60 folders, 2 sessions), `torii` (113, 2),
`gateway` (90, 1) are fresh projects; `dbd-rs` is an orphaned empty project (0/0);
`llm-gateway` (25, 0) is a stray; 13 `transcript_cursor` rows + prose in
`transcript_turns` (790 total) point at the old paths and don't resolve.

## Design — 4 parts

### 1. Path-alias retention (the "don't dangle" core)
NEW `sensei.folder_path_aliases (alias_abs_path text unique, folder_id uuid → folders(id) on delete cascade, reason, created_at)`. Extend the two resolvers to match an alias when `abs_path` misses:
- `get_folder_ids_by_path`: `WHERE abs_path = $1` → also `OR $1 IN (aliases)`.
- `find_folder_for_path`: ancestor match on `abs_path` **or** any alias.
A folder keeps its identity (uuid, project, sessions) across a path change; old
transcripts resolve through the alias.

### 2. Rename detection → auto-remap (so "on rename" is automatic)
- Populate `folders.remote_urls` at scan (read `git remote get-url origin` for a
  `kind='git'` folder — the column already exists; `process.rs` just never fills it).
- In reconcile, before `Remove`-ing a vanished root, check for a freshly-discovered
  folder with the **same remote** → it's a rename → **remap**: move the old folder's
  identity onto the new `abs_path`, record the old path (+ descendant paths) as
  aliases. Preserves project + sessions + history with no user action.
- No remote match (local-only repo, no origin) → fall through to part 3.

### 3. Archive, don't delete (the "transcripts but no repo" case)
When a folder truly vanishes with no rename match, **do not hard-delete**. Mark the
folder + its project `archived` (retain the rows, the sessions, and the transcript
links). The project surfaces as archived/closed. Reuses the existing `stale`/
`orphaned` tagging path (`mark_orphaned_projects`, `scan_logic.rs classify_stale_root`)
but swaps the destructive `delete_folder_tree`/`prune_empty_projects` on
history-bearing folders for an archive transition. (Genuinely empty, history-less
discovery folders can still be pruned — no data to lose.)

### 4. Explicit remap command (deterministic; drives the data correction + manual moves)
`sensei folder remap <old-abs-path> <new-abs-path>` (CLI + a daemon API/task):
retarget the folder's `abs_path` (+ descendant folders by prefix) to the new path
and register the old path(s) as aliases; re-run transcript synthesis for the
affected sessions so history attributes to the current project. Handles moves the
daemon can't auto-detect (no git remote) and is the tool for the correction below.

## Data correction (live `sensei` DB — needs explicit approval)
1. Aliases so the orphaned transcripts resolve to the current folders:
   `…/Developer/dbd-rs` → the `dbd` folder; `…/Developer/strategos/monorepo` (+ its
   `docs/mockups`, `docs/mockups/app` subpaths) → the `torii` folder;
   `…/Developer/strategos/gateway` → the `gateway` folder.
2. Fold the orphaned empty `dbd-rs` project into `dbd` (or archive it); investigate
   `llm-gateway` (25 folders, 0 sessions — likely the old strategos/gateway project
   under a frontmatter name) and merge/archive.
3. Re-run transcript synthesis for the 13 cursored old sessions → their history
   attaches to dbd/torii/gateway.

## Staging
- **Stage 1 (unblocks the correction):** alias table + resolver + explicit remap +
  archive-on-vanish (stop the data loss). Then apply the data correction.
- **Stage 2 (auto):** populate `remote_urls` at scan + remote-based auto-remap in
  reconcile, so future renames self-heal with no command.

## Tests
- Pure: alias-aware resolution; remap prefix-rewrite of descendant paths; the
  rename-vs-delete decision (remote match → remap, else archive).
- DB-integration (existing `pg_store` test harness on `sensei_test`): `get_folder_ids_by_path`
  matches via alias; remap preserves folder id + sessions; archive retains sessions
  (no cascade delete).

## Decisions to confirm
1. **Rename auto-detection = git remote** (Stage 2) — OK to read `git remote` at
   scan + populate `remote_urls`? (The alternative is explicit-remap-only.)
2. **Archive-not-delete** changes a shipped destructive reconcile path — confirm we
   retain (archive) history-bearing vanished folders rather than delete them.
3. **Apply the data correction now** against the live `sensei` DB (with a psql
   transaction), or land Stage 1 code first and correct via the new `remap` command?
