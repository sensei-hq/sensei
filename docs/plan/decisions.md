# Decisions log — considered · adopted · discarded · deferred

> When a doc is superseded and archived, the *ideas* worth keeping land here so
> nothing useful is lost and nothing already-rejected gets re-proposed. This is
> the memory of **why the shape is the shape**. Additive — append, don't rewrite
> history. Pairs with [the plan](README.md)
> (what's left to build) and [`backlog.md`](../backlog.md) (the issue index).

## Adopted — concepts carried forward (salvaged before archival)

| Concept | Where it lives now |
|---|---|
| The **four-segment journey** (Bootstrap · First-run+Prefs · Observatory · Project) | [requirements/vision.md](../requirements/vision.md), [objectives.md](../requirements/objectives.md) |
| **Value before setup** — projects first, not a wizard | vision theme 1; realised as first-run-scan + Preferences |
| The **module lifecycles** (the loops inside the daily app) | objectives O5 |
| **FTR** as the single north-star | vision.md |
| **Adapter-IR** + language-adapter split; task hierarchy + barriers; compression L0–L3 | [architecture/daemon.md](../architecture/daemon.md) |
| **Single binary, single DB, port 7744** | architecture/data.md + daemon.md |
| The **retrospective-loop** framing (capture→graph→analyze→learn→deliver→measure) | vision.md core loop |
| Dōjō **priority-ladders / specificity-wins / pull-never-push / preview-always** | [architecture/dojo.md](../architecture/dojo.md), objectives DJ* |

## Adopted — decisions with rejected alternatives (ADRs)

The *why*, so a rejected shape isn't re-proposed. Salvaged from the old `design/decisions/` ADRs + `ideas/`.

| Decision | Rejected alternatives (why) |
|---|---|
| **One repo = one project = one owner** (#101) | promoting every walked subfolder/crate to a project — double-indexes and gives folders their own code nodes. Crates/packages are **structural members with a role**; only a genuine nested `.git` subtree is a separate owner. Enforced at classification + self-healed by `dedup_structural_folder_nodes`. |
| **Postgres + pgvector as the single store** (:7744) | SQLite+Kuzu (no cross-store txns, N+1 on JSON-array joins, Kuzu unmaintained/embedded-C++ risk, no vectors); FalkorDB/Chroma/MemPalace (need a server process — wrong for a zero-server desktop); in-memory queue (lost task state on restart). Postgres unifies relational + graph + vectors + a durable queue with concurrent app/MCP/CLI connections. |
| **Code is a graph → relational tables + recursive CTEs** | a flat vector index or regex extraction — destroys the call/import/type graph; cosine surfaces files that *mention* similar words but can't answer "who calls X". Recursive CTEs cover 90%+ (1–2 hop); Apache **AGE** can layer on the *same* tables later with no migration. |
| **Adapter-IR = three node types** (`IRDoc`/`IRModule`/`IRClass`), all `Option<>` | one unified symbol type — structure differs per kind. `Option` everywhere so missing frontmatter/annotations degrade gracefully; per-file parse is worker-parallel, edge/parent resolution is a separate batch phase. |
| **Preferences merge INTO memory** | a separate `inference.preferences` table — memories are part of the graph, not a silo. The table was killed. |
| **Agent-agnostic MCP surface** | per-assistant tool variants — per-assistant differences live in a coordinator *adapter*, never in the tool contracts. The AI only ever reaches the daemon via MCP. |
| **Vertical feature slices** (daemon→MCP→command in one issue, innermost-first, fully tested before the next layer) | per-layer issues + stubs — they cause rework cycles (D18). |

## Discarded — considered and rejected (do not re-propose)

| Idea | Why rejected | Superseded by |
|---|---|---|
| **10-step linear setup wizard** (old `ideas/02-setup.md`) | A first-run gauntlet violates *value before setup*; users abandoned it | first-run-scan (value immediately) + a searchable **Preferences** surface |
| **dev/prod split** (two binaries, port 7745) | Needless complexity; one machine, one dataset | **single mode** — one binary, one DB, :7744 |
| **Gateway as an in-tree crate** (`crates/gateway/`) | Couldn't release to crates.io independently | **`gateway-embedded`** git dep (sibling repo `sensei-hq/gateway`) |
| **`hive-mind` / `sensei-hive` naming** | Ambiguous; drifted from the product concept | **Dōjō / `sensei-dojo`** (`crates/dojo-mind`) — terminology migration in progress |
| **Push-based federation** | Leaks control + confidentiality | **pull, never push** + preview-always (theme 5) |
| **Global collective as the default share lane** | Org/client work must not hit the public commons | the **Dōjō membership** boundary is the default; Collective is explicit opt-in |
| **`client-lead` role name** | Longer than needed; the engagement (not the person) is the "client" | role renamed to **`lead`**; it still guards *client engagements* |

## Deferred — good, not now (parked with a trigger)

| Idea | Why deferred | Revisit when |
|---|---|---|
| **User-facing learnings** ("you tend to give sparse instructions for schema changes…") | The pair-both-ways signal needs the loop closed first | after Phase 1 (FTR loop closes) |
| **Assistant proactive clarification** ("I need the migration policy before I can answer") | v2 behaviour; needs the signal + a prompting contract | [pipeline/clarification-prompting](../spec/pipeline/clarification-prompting.md) — post-Phase 2 |
| **Benchmarks** (runner + corpus) | No runner/TaskKind yet; not on the FTR path | when model-effectiveness needs a controlled corpus |
| **Testability** (test-runner adapter → `test_pass_rate`) | No test-runner integration; quality signal optional | when a quality dimension is prioritised |
| **Diagnostic sessions/traces + issue export** (#39) | Larger new-schema + cross-cutting capture effort; only flat `public.logs` today | when support/debug UX is prioritised |
| **Image-gen as seed** (#77) · **embedded in CI release binaries** (#78) | Need `model_capability=image` / cross-platform native llama.cpp sign-off | gateway/seed hardening pass |
| **Dōjō live activation** | External-blocked (needs a remote server + SaaS-infra decision) | Phase 4 |
| ~~ACP + control-plane / relay~~ **PROMOTED to an active vision 2026-07-14** | adopt Apache-2.0 **ACP**, not Zed's GPL agent crate; agents on your hardware, filtered status only, daemon outbound-only | now specced: [requirements](../requirements/vision.md#the-relay--long-runs-supervised-from-anywhere) R1–R8 + [architecture/relay](../architecture/relay.md) + a plan phase. Still future to *build* |
| **Response cache** (semantic cache of notable LLM outputs — TTL/pin/secret-scrub, ≤3 session-start hints) | distinct from the memory pipeline; never built | when prompt/response reuse becomes a cost lever |
| **Telemetry transactional-outbox** (durable event write-buffer + backoff/eviction + queue-depth health) | accepted direct-write today | when "no events lost during downtime" is prioritised |
| **Data-source connectors** (Confluence/Jira/Notion/Figma as graph-node producers — "the graph doesn't care whether a node came from git or Confluence") | repo-only today | when Solution scope needs non-repo members |
| **Cursor as 2nd coordinator** + generic capture fallback ladder (MCP/file-watch/poll/OTLP for tools without hooks) | Zed shipped first; Cursor = largest user base | when a 2nd coordinator is prioritised |
| **Capability Registry** (per-ACP data-availability Real/Workaround/Unavailable + `discard_when` upstream-issue + workaround lifecycle) | keeps workaround code inert until upstream lands, then auto-cleaned | when cross-ACP graceful degradation is built |
| ~~`llm-spec/` → `spec/` rename~~ **DONE 2026-07-14** | was high-churn (run-state/cron driver + gate agents + memory) | completed: dir renamed + all referrers fixed (docs, `.claude/agents`, code comments) |

## How to use this

- Rejecting an idea? Add a **Discarded** row with the reason — future-you (and the
  next assistant) won't waste a cycle re-proposing it.
- Parking an idea? Add a **Deferred** row with an explicit *revisit-when* trigger.
- Promoting a deferred idea to active work? Move it into
  [open-issues.md](README.md) and note the date here.
