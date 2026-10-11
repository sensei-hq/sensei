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
| ~~249~~ | relay owner-only | CLOSED `ada807a2`; peer status view → #252 | — |
| 1. 238 | Claude Code mods (3 of 4 done; replay mis-pairing 69%→0 `d1196ed1`) | unblocked | register `PostToolUseFailure`; observe a non-zero Bash first; then `success` from event type |
| 3. 251 | schema store + indexer routing | #221 prerequisite | rewrite spec 20 → DDL → `SchemaFacts` routing → dbd importer |
| 4. 221 | Schema / ER screen | needs 251 steps 1–5 | `ErDiagram` over the store |
| 5. 236 | dōjō access: views + RLS, user session | large; kavach#51 upstream | access-matrix doc |
| 6. 225 · 239–241 · 245 | team view · productionise · e2e red gate | — | — |

**Next command:** #238 — observe which event a non-zero Bash exit fires, then register `PostToolUseFailure`.

**Note:** `install-service` deploys the RELEASED DDL; a view/table change reaches the live DB only via a release or by hand (`psql -f`), as done for `session_tool_calls`. Untracked files from another session in the tree (`src/`, `docs/components/`, `docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Data:** the leftover orphan rows stay until the full re-index after the
indexing fixes land (owner decision 2026-10-08).

**Known broken:** e2e #245. Marketplace hook fixes unpublished until `make
bump`. Old hashes in docs/*.md (22 refs, 10 files) predate the rewrite.
