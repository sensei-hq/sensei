# develop → main merge runbook (code-graph idempotency ship)

> Status: **EXECUTED 2026-08-07 → released as v0.7.0.** `develop` (76 commits
> ahead + the FQN symbol-table rebuild, Phases 1–7, #108, on top of the original
> code-graph idempotency work) merged to `main` (merge `5e83fcca`), bumped
> `0.6.4 → 0.7.0` (`make bump v=minor`; tag pushed, homebrew-tap + marketplace
> synced), release binary installed (`make install-service`). Live deploy gate
> applied: graph-cleared + `dbd reconcile --scope default` (adds
> `edges_target_id_idx` + accumulated additive drift; FQN gates from
> `2026-08-07-fqn-live-migration.sql` already present) → full reindex of all
> 8,664 folders running on the 0.7.0 daemon.
>
> Original pre-merge status (2026-08-06): ready to merge; 50 commits ahead; 0.6.4.

## What ships

The complete **code-graph idempotency** feature (issue #108, plan
`docs/plan/2026-08-05-code-graph-idempotency-plan.md`), Phases 0–7:

- **D1** edge identity + idempotent `insert_edge`/`resolve_edge`
- **D2** replace (not append) derived edge sets, transactionally
- **D3** upsert-then-prune node identity + embedding survival
- **D4** durable/deterministic communities: coverage (~100%), degree, god-nodes,
  description-via-insight-copy (non-load-bearing, honest-null)
- **D5a** `workspace_member`/`subtree` folder kinds; **D5b** `section`/`rationale`
  doc nodes (nested, line-independent identity)
- **D6** worker robustness: single-writer, folder-status lifecycle, bounded
  retry, fail-closed barriers
- **Phase 7** retrieval contract: `graph/nodes` (community_id + structural edges),
  new `GET /api/graph/{repoId}/tree`, live `communities/info`, whole-graph
  integration test + committed fixture

Plus: qlty dedup (shared test `make_ctx`), the Dependabot lockfile fix
(`f2a1f809`), bootstrap health-test fixes, and app F8 (shared ScreenState).

## Pre-merge gates — all met

- ✅ **Zero-errors** per commit (`cargo clippy --all-targets` clean, `make
  test-fast` on every commit via pre-commit).
- ✅ **Adversarial correctness reviews** on D3, D4, Phase 6, Phase 7 — every
  finding fixed and re-verified (2 D4 Criticals, 1 P6 Critical, 1 P7 Important).
- ✅ **Live verification** (2026-08-06): migrated the live `sensei` DB, reindexed
  `~/Developer`+`~/Work`, confirmed on sensei/torii/dbd — `section`+`rationale`
  nodes, `workspace_member` folders, section nesting, 100% community coverage,
  `/tree` + `communities/info` endpoints, folders reaching `indexed`.
- ✅ **Dependabot**: both alerts (quinn-proto HIGH, serde_with MED) already
  patched on `develop`; they auto-dismiss when `main` receives the fix.

## Merge steps

1. **Merge `develop` → `main`** (no rebase — preserve the reviewed history):
   ```bash
   git checkout main && git pull && git merge --no-ff develop && git push origin main
   ```
   (Dependabot alerts close once `main` has the patched lockfile.)

2. **Bump the version** — the release trigger (updates all manifests, tags,
   pushes, syncs the homebrew + marketplace subtrees):
   ```bash
   make bump v=minor   # 0.6.4 → 0.7.0  (feature release: code-graph idempotency)
   ```
   Use `minor` — this is a substantial feature, not a patch.

## ⚠ Deploy gate — REQUIRED on every live DB upgrading to this version

The new binary's DDL (edges partial unique indexes + `inference.communities.props`)
cannot land on a live DB that still holds duplicate edges or lacks the column.
**This was executed on the maintainer's DB 2026-08-06; any OTHER live install
upgrading past 0.6.4 must run the same one-time migration** (documented in
`docs/backlog.md`):

```sql
-- 1. clear the dup-laden graph (dup edges block the new unique index)
TRUNCATE sensei.edges, sensei.nodes CASCADE;   -- cascades inference.drift_items
TRUNCATE inference.communities;
TRUNCATE sensei.scan_state;                     -- so the re-scan is a full rebuild
UPDATE sensei.folders SET status='discovered'   -- re-drive every folder
  WHERE status IN ('indexed','indexing','failed');
```
```bash
# 2. reconcile the new DDL (adds communities.props; edges indexes build on empty tables)
cd database && dbd reconcile -d "$DATABASE_URL"
# (edges unique indexes are inline in edges.ddl — create manually if reconcile skips them:
#  create unique index if not exists edges_unique_resolved on sensei.edges
#      (folder_id, source_id, target_id, kind) where target_id is not null;
#  create unique index if not exists edges_unique_unresolved on sensei.edges
#      (folder_id, source_id, target_name, target_file, kind) nulls not distinct where target_id is null;)
# 3. restart the daemon (new binary) → it re-scans watch roots → rebuild + re-derive traceability
```

Fresh installs (empty DB) need none of this — the schema materializes clean.

## Post-merge follow-ups (tracked in docs/backlog.md, not merge-blocking)

- Systemic **watchdog-abort fail-closed** gap (a barrier task killed by the
  watchdog doesn't write `folder_status='failed'`) — low-pri, self-heals on boot.
- **D5b code-comment rationale** + finer parenting (currently doc-path only).
- **D5c** package/sub-symbol node emission (P2, cut from this plan).
