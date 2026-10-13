# Placement report — sensei, 2026-10-13

What share of this repository's call and reference edges the indexer places,
by language, crate/package and app, after the compiler-engine cycle
(`docs/plan/2026-10-13-indexer-full-call-placement.md`). Two measurements, one
path: the live graph the daemon wrote (final forced rescan, t0
`2026-10-13T13:18:25Z`), and the placement floor
(`indexer::quality::placement`), which compiles the repository with the same
engines and places every file through the same `index_file`.

A use that names no declaration — a local, a parameter, an anonymous
function, a value's member, an index entry — is no edge, so it is neither
placed nor missed. Imports stay with the ladder (99.8% placed, unchanged).

## By language — the live graph

Edges of files parsed in the final rescan (`sensei.edges`, deduplicated as
the graph stores them).

| language | calls placed | calls missed | calls | references placed | references missed | references |
|---|---:|---:|---:|---:|---:|---:|
| Rust | 63,906 | 0 | **100.00%** | 42,626 | 35 | **99.92%** |
| TypeScript/JavaScript | 10,542 | 4 | **99.96%** | 10,468 | 71 | **99.33%** |
| Svelte | 1,777 | 1 | **99.94%** | 2,640 | 12 | **99.55%** |
| SQL (DDL, via dbd) | 382 | 0 | **100.00%** | 453 | 0 | **100.00%** |

Before this cycle the same graph placed Rust calls at 44.3% and TS/JS/Svelte
calls at 44.9%, through the ladder.

## By crate, package and app — the live graph

`sensei.resolution_quality(project)`, grouped by the source node's package
(fqn segment 2 — the crate or the app), calls and references.

| language | package | crate / app | edge kind | placed | missed | placed % |
|---|---|---|---|---:|---:|---:|
| rust | `dojo-protocol` | crate | calls | 255 | 0 | 100.00% |
| rust | `dojo-protocol` | crate | references | 626 | 0 | 100.00% |
| rust | `graph-scan` | crate | calls | 1 | 0 | 100.00% |
| rust | `sensei-bootstrap` | crate | calls | 1,880 | 0 | 100.00% |
| rust | `sensei-bootstrap` | crate | references | 1,620 | 0 | 100.00% |
| rust | `sensei-cli` | crate | calls | 842 | 0 | 100.00% |
| rust | `sensei-cli` | crate | references | 405 | 0 | 100.00% |
| rust | `sensei-config` | crate | calls | 12 | 0 | 100.00% |
| rust | `sensei-config` | crate | references | 1 | 0 | 100.00% |
| rust | `sensei-desktop` | desktop app shell (app/src-tauri) | calls | 587 | 0 | 100.00% |
| rust | `sensei-desktop` | desktop app shell (app/src-tauri) | references | 297 | 0 | 100.00% |
| rust | `sensei-logger` | crate | calls | 84 | 0 | 100.00% |
| rust | `sensei-logger` | crate | references | 171 | 0 | 100.00% |
| rust | `sensei-mcp` | crate | calls | 527 | 0 | 100.00% |
| rust | `sensei-mcp` | crate | references | 154 | 0 | 100.00% |
| rust | `sensei-transcript-formats` | crate | calls | 143 | 0 | 100.00% |
| rust | `sensei-transcript-formats` | crate | references | 60 | 0 | 100.00% |
| rust | `senseid` | crate | calls | 58,529 | 0 | 100.00% |
| rust | `senseid` | crate | references | 38,545 | 35 | 99.91% |
| rust | `session-report` | crate | calls | 1,046 | 0 | 100.00% |
| rust | `session-report` | crate | references | 775 | 0 | 100.00% |
| sql | `sensei` | repo DDL | calls | 382 | 0 | 100.00% |
| sql | `sensei` | repo DDL | references | 453 | 0 | 100.00% |
| typescript | `@sensei/desktop` | desktop app (app/) | calls | 6,486 | 3 | 99.95% |
| typescript | `@sensei/desktop` | desktop app (app/) | references | 7,897 | 56 | 99.30% |
| typescript | `sensei-dojo-web` | dōjō (dojo/) | calls | 5,453 | 0 | 100.00% |
| typescript | `sensei-dojo-web` | dōjō (dojo/) | references | 4,752 | 23 | 99.52% |
| typescript | `sensei-site` | website (website/) | calls | 137 | 0 | 100.00% |
| typescript | `sensei-site` | website (website/) | references | 333 | 1 | 99.70% |
| typescript | `sensei-ts-driver` | TS driver (crates/senseid/ts-driver) | calls | 236 | 2 | 99.16% |
| typescript | `sensei-ts-driver` | TS driver (crates/senseid/ts-driver) | references | 126 | 3 | 97.67% |
| typescript | `senseid` | test fixtures | calls | 7 | 0 | 100.00% |

## The floor — every occurrence, test code included

`cargo test -p senseid --bin senseid placement_never_falls_below_the_floor --
--ignored --nocapture` (nightly: `.github/workflows/placement-floor.yml`).
Counts every reference occurrence rather than deduplicated edges, so its
numbers are larger than the graph's; every (language, package, kind) cell must
stay at or above 90% — all pass.

| language | kind | placed | missed | share |
|---|---|---:|---:|---:|
| rust | calls | 100,967 | 0 | 100.00% |
| rust | references | 46,272 | 41 | 99.91% |
| svelte | calls | 2,551 | 1 | 99.96% |
| svelte | references | 4,280 | 14 | 99.67% |
| ts/js | calls | 36,752 | 5 | 99.99% |
| ts/js | references | 11,911 | 100 | 99.17% |

## What is not placed, and why

Every remaining miss carries its root cause (`sensei.reason_codes`):

- **`untyped_receiver`** — the source types the receiver `any`/`unknown` (a
  helper parameter, `JSON.parse`, d3-zoom's own `event: any`, an untyped
  shim), so nothing static names the callee. Fixable in the source, not the
  indexer: typing the e2e helpers and the TypeScript driver took this from
  ~480 to ~90 occurrences in this cycle.
- **`macro_expansion`** (Rust, 25 edges) — uses of declarations a declarative
  macro generates (`schema_vocabulary! { SchemaSource { … } }`); the walk
  reads the invocation as text and does not expand it. A refusal, stated.
- **`dynamic_dispatch`** (1) — `this[event.action](…)`: a member chosen at
  run time.
- **`compiler_no_symbol`** (Rust, 9) — type annotations on `let` bindings
  inside `#[tokio::test]` bodies and fn-local types (`Site`, `Check`) where
  rust-analyzer's SCIP output holds no occurrence for the token.
- **`no_import_in_scope`** (Rust, 1) — a ladder verdict on a relation edge.
- **Known stale data, not a placement verdict:** 3 C reference edges from 4
  file-less member nodes left by the tree-sitter grammar headers, which are now
  excluded as vendored (`.gitattributes`). A skipped file sheds the nodes it
  owns by file; nodes that carry no file are not yet swept (backlog).

## The diagrams' coverage lines agree

Read from the daemon's own diagram endpoints after the final rescan:

| diagram | coverage line | agrees because |
|---|---|---|
| Structure (`/diagrams/structure`) | 2,456 drawn · **5 unplaced** | the 5 are exactly the unplaced `calls` above (Svelte 1 + TS/JS 3 `untyped_receiver`, 1 `dynamic_dispatch`) |
| Layering, module grain (`/diagrams/layering?level=module`) | 138 units · 238 dependencies drawn · 5 unplaced · 2 with an endpoint that owns no file | the same 5; the 2 are dependencies whose endpoint is a file-less node |
| Schema / ER (`/diagrams/schema`) | 114 entities · 189 refs | every foreign key Postgres knows (121) is in the store; the other 68 are declared in the DDL for objects the local database does not carry (dōjō's tables deploy to Supabase) — none missing |
| Zones (`/diagrams/zones`) | 138 points · 49 unplaceable | a different measure: Martin's distance from the main sequence is undefined for a component with no types or no couplings — not an unplaced edge |

## Reproduce

- Live verdict by language: `docs/reports/2026-10-13-placement.md` tables come
  from `sensei.resolution_quality('<project id>')` and `sensei.graph_placement`.
- Floor: the `cargo test` command above (needs rust-analyzer + rust-src, node,
  the front ends' dependencies, and `~/.sensei/engines`, which bootstrap installs).
