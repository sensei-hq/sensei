# Checkpoint

**State 2026-10-08.** History rewritten to Jerry Thomas (all refs force-pushed,
`main` protection restored). Open PRs: none. Branches: main + develop only (WIP diffs in ~/sensei-history-backup/wip-patches-20261008). 108 → 101 open issues this run.

| # | item | status | to close |
|---|---|---|---|
| 220 219 231 233 218 248 202 203 205 232 | done | CLOSED | — |
| — | CodeQL for Rust | MERGED #250 | baseline at next main merge |
| 247 | pruner + root removal | CLOSED — orphan cleanup dropped; data recreated after the fixes | — |

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

**Data:** the leftover orphan rows stay until the full re-index after the
indexing fixes land (owner decision 2026-10-08).

**Known broken:** e2e #245. Marketplace hook fixes unpublished until `make
bump`. Old hashes in docs/*.md (22 refs, 10 files) predate the rewrite.
