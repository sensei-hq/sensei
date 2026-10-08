# Checkpoint

**State 2026-10-08.** History rewritten to Jerry Thomas (all refs force-pushed,
`main` protection restored). Open PRs: none. Branches: main + develop only (WIP diffs in ~/sensei-history-backup/wip-patches-20261008). 108 → 101 open issues this run.

| # | item | status | to close |
|---|---|---|---|
| 220 219 231 233 218 248 202 203 205 232 | done | CLOSED | — |
| — | CodeQL for Rust | MERGED #250 | baseline at next main merge |
| 247 | pruner + root removal | gates 1–5 done | gate 6: user runs the SQL below |

**Queue, in order** (re-sequenced 2026-10-08 after the #221/#236/#249 decisions):

| # | item | why here | first step |
|---|---|---|---|
| 1. 249 | relay: owner-only; peers see status only | live exposure, small | scope 5 relay routes to `caller.membershipId`, route tests A≠B |
| 2. 238 | Claude Code mods (2 of 4 done) | small, unblocked | harvest `duration_ms` / `agent_id` |
| 3. 251 | schema store + indexer routing | #221 prerequisite | rewrite spec 20 → DDL → `SchemaFacts` routing → dbd importer |
| 4. 221 | Schema / ER screen | needs 251 steps 1–5 | `ErDiagram` over the store |
| 5. 236 | dōjō access: views + RLS, user session | large; kavach#51 upstream | access-matrix doc |
| 6. 225 · 239–241 · 245 | team view · productionise · e2e red gate | — | — |

**Next command:** #249 — scope `dojo/src/routes/v1/t/[origin]/[org]/relay/{gates,reply,segments,session,review}` to the caller's membership, test-first.

**Orphan cleanup the classifier blocked** (user to run; newest backup
`database/backup/essential/20261007-200259`):
`DELETE FROM sensei.repositories r WHERE NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.repository_id = r.id) AND r.synced_at IS NULL AND r.tenant_id IS NULL AND NOT EXISTS (SELECT 1 FROM sensei.repositories_in_projects rp WHERE rp.repository_id = r.id);`
→ 7,795 rows.

**Known broken:** e2e #245. Marketplace hook fixes unpublished until `make
bump`. Old hashes in docs/*.md (22 refs, 10 files) predate the rewrite.
