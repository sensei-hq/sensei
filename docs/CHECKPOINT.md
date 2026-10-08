# Checkpoint

**State 2026-10-08.** History rewritten to Jerry Thomas (all refs force-pushed,
`main` protection restored). Open PRs: none. Branches: main + develop only (WIP diffs in ~/sensei-history-backup/wip-patches-20261008). 108 → 101 open issues this run.

| # | item | status | to close |
|---|---|---|---|
| 220 219 231 233 218 248 | diagrams, consent, e2e pointer | CLOSED | — |
| 202 | gateway v0.7.0 | CLOSED — PR #228 merged `ec48c565` | — |
| 203 205 232 | llama build, Structure, Layers/Cycles | CLOSED — re-verified live | — |
| — | CodeQL for Rust (archived PR) | MERGED as #250 `1e177907` | baseline appears at next main merge |
| 247 | pruner + root removal | gates 1–5 done | gate 6: user runs the SQL below |
| 238 | Claude Code mods | 2 of 4 defects fixed | `duration_ms`/`agent_id` harvest, `success` column, mods 1–5 |
| 221 | Schema diagram | DECIDED C — light nodes + separate schema tables; dbd importer + ORM extractors | revise spec 20 → DDL → importer → ER screen |
| 236 | dōjō data access | DECIDED B — views + RLS, user session; kavach#51 filed | access matrix doc → views+RLS → routes → device-token JWT |
| 249 | relay access | DECIDED — owner-only; peers see status only | scope 5 relay routes to caller; status-only peer view |
| 225 | dōjō team view | queued | sync-shape decision |
| 239 240 241 | productionise epic | queued | — |
| 245 (+134 dup) | e2e red gate | open | 4 groups, one commit each |

**Next command:** #238 — harvest `duration_ms` / `agent_id` from PostToolUse.

**Orphan cleanup the classifier blocked** (user to run; newest backup
`database/backup/essential/20261007-200259`):
`DELETE FROM sensei.repositories r WHERE NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.repository_id = r.id) AND r.synced_at IS NULL AND r.tenant_id IS NULL AND NOT EXISTS (SELECT 1 FROM sensei.repositories_in_projects rp WHERE rp.repository_id = r.id);`
→ 7,795 rows.

**Known broken:** e2e #245. Marketplace hook fixes unpublished until `make
bump`. Old hashes in docs/*.md (22 refs, 10 files) predate the rewrite.
