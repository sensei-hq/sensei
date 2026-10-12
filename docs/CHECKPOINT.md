# Checkpoint

**State 2026-10-11.** #251 steps 1–5 landed and verified LIVE (`a12f2710` spec, `12d9d3bc` DDL, `d231b5dc` routing, `60549739` importer + `c15a3645` DDL fix, `91a4350e` endpoint): 169 tables/184 refs/784 deps, deps identical to dbd's own. Earlier, 2026-10-08: History rewritten to Jerry Thomas (all refs force-pushed,
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
| 238 | Claude Code mods | 4 of 4 live defects FIXED (`c40cb02b`); Steps 2–5 need the mods API | settle mods `.d.ts`, then usage-at-source |
| 1. 251 | schema store + indexer routing | #221 prerequisite | steps 1–5 DONE + live; step 7 EF Core extractor remains |
| 2. 221 | Schema / ER screen | UNBLOCKED | `ErDiagram` over `/diagrams/schema` |
| 3. 236 | dōjō access: views + RLS, user session | large; kavach#51 upstream | access-matrix doc |
| 4. 225 · 239–241 · 245 · 252 | team view · productionise · e2e red gate | — | — |

**Next command:** #221 — add `api.ts` client for `GET /api/projects/{id}/diagrams/schema`, then the Schema screen: rokkit `ErDiagram` (`toGraphInput(model,'er')`) + entity/deps views; spec first (`docs/spec/screen/`).

**Note:** `install-service` deploys the RELEASED DDL; a view/table change reaches the live DB only via a release or by hand (`psql -f`), as done for `session_tool_calls`. Untracked files from another session in the tree (`src/`, `docs/components/`, `docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Data:** the leftover orphan rows stay until the full re-index after the
indexing fixes land (owner decision 2026-10-08).

**Known broken:** e2e #245. `schema_*` tables + `import_dbd_schema` applied to live and sensei_test BY HAND; installs need a release. SQL reader ignores design.yaml's stated dialect (noted on #251). Marketplace hook fixes unpublished until `make
bump`. Old hashes in docs/*.md (22 refs, 10 files) predate the rewrite.
