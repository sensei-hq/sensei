# Gap register

The consolidated, IDed list of every known gap across the product — one line,
its evidence, its priority, and the module it blocks. This replaces the old
plan gap analysis's G1–G10 numbering and the `production-readiness.md` A–H
(now `07-quality.md`), which are absorbed here.

> **Status:** the register's full population is **Phase 3** of the reorg
> ([`../reorg.md`](../reorg.md) §12, §13). The ID scheme is final; the rows
> below are the ones already decided (2026-10-13) and the ones the plane files
> rely on. The complete F-/P-/T-/D-/M-/C-/X- register is folded in from reorg §12.

## ID scheme

| Prefix | Family |
|---|---|
| `F-*` | faults / silently-wrong behaviour |
| `P-*` | performance / scale |
| `T-*` | tests / gates / e2e |
| `D-*` | docs / claims / website truth |
| `M-*` | metrics / health / gating |
| `C-*` | capture / conventions / adapters / personas |
| `G-*` | graph / visualisation |
| `ACP-*` | relay / ACP agent runtime |
| `X-*` | cross-cutting / docs-repo |

Every row: **ID · module · one line · evidence · priority.** A gap with no
evidence is a suspicion, and gets labelled so.

## Decided gaps (2026-10-13)

| ID | Module | One line | Evidence | Priority |
|---|---|---|---|---|
| ~~A-1~~ | M2 · M17 | ~~app is kanji-only~~ — the kanji ⇄ icons switch is live (`ui.glyphs`, Settings → General; #133) | decision 10; reorg §12.10 | — |
| A-2 | M2 | the personal-preference surface is narrower than the config the app already supports | decision 10 | P3 |
| G-1 | M17 | graph/layered view unreadable at granular scale — no L0/L1/L2 level of detail | reorg §9.7 | P1 |
| G-2 | M4 · M2 · M3 | indexer has no ignore mechanism (no `.senseiignore` reader, no ignore config); it walks committed mockups / vendored / generated code and skews the graph + metrics. Ignore is stored repo config synced via the Dōjō, **seeded from the repo's own ignore sources** (`.gitignore`, qlty `.qlty/qlty.toml` `exclude_patterns`) — not a dotfile | decision 2026-10-13 | P2 |
| M-7 | M8 | no master-detail metric model, no `gating` field, gating eligibility undefined | reorg §9.6 | P1 |
| M-8 | M8 | coverage-gated presentation absent; thin metrics read as strong (#155 generalised) | reorg §9.6 | P1 |
| M-9 | M8 | no per-metric design/coverage record tying a metric to its chart and LOD | reorg §9.6 | P1 |
| C-11 | M3 | transcript-scanning review not scoped as a task | decision 10 | P1 |
| C-12 | M5 · M14 | inference-pipeline review not scoped as a task | decision 10 | P1 |
| C-13 | M3 | Claude "mods" (supersedes plugin + hooks) — install path + migration not verified | decision 10 | P1 |
| C-14 | M3 | multi-assistant expansion: Claude + OpenCode priority; others later; seam unspecified | decision 10 | P1 |
| C-15 | prd | personas (incl. manager + client/stakeholder viewpoints) — ✅ authored 2026-10-13 | `../prd/03-personas.md` | done |
| C-16 | M3 | **Copilot CLI: `SynthEvent` lacks `tool_success` field** — `tool.execution_complete` reports `success: false` (2.2% failure rate in sample) but it cannot be represented; the only direct measure of agent friction is discarded | 2026-08-26 review §Signals we are still discarding | P1 |
| C-17 | M3 | **Copilot CLI: turn duration not captured** — `assistant.turn_start` → `turn_end` available but not parsed | 2026-08-26 review §Signals we are still discarding | P2 |
| C-18 | M3 | **Copilot CLI: `codeChanges` from `session.shutdown` not captured** — real lines added/removed and files touched | 2026-08-26 review §Signals we are still discarding | P2 |
| C-19 | M3 | **Copilot CLI: per-model token usage with cache read/write not fully captured** — `session.shutdown` carries `totalPremiumRequests`, cache tokens; partially parsed but not all fields | 2026-08-26 review §Signals we are still discarding | P2 |
| C-20 | M3 | **Copilot CLI: `session.permissions_changed` not captured** — every point the agent stopped and waited for a human | 2026-08-26 review §Signals we are still discarding | P3 |
| C-21 | M3 | **VS Code: no tool outcome recorded** — `toolInvocationSerialized` says a tool ran, never whether it worked; failure rate computed from it is 0/N (reads as flawless) | 2026-08-26 review §What VS Code does not record | P2 (known limitation) |
| C-22 | M3 | **VS Code: no tokens of any kind recorded** | 2026-08-26 review §What VS Code does not record | P2 (known limitation) |
| C-23 | M3 | **VS Code: turn latency not measurable** — `responseTimestamp` usually equals `timestamp`, elapsed time is not measurable | 2026-08-26 review §What VS Code does not record | P2 (known limitation) |
| C-24 | M3 | **Cursor adapter: no real sample data to verify against** — the only adapter with no real data behind it | 2026-08-26 review §Cursor, §Suggested order | P1 |
| C-25 | M3 | **Usage & cost contract not met beyond Claude Code** — the M3 adapter contract (token split, model, billed vs list cost, per-turn usage, context, headroom) is met in full only by Claude Code (transcript + usage mod); OpenCode lacks per-turn/context, Zed and VS Code report no cost, Copilot CLI is partial (C-19), Cursor reports nothing (C-24) | `prd/modules/01-foundation/03-capture-sessions` §Usage & cost capture; #238 | P2 |
| ACP-1 | M21 | No ACP-native driver for Claude (hooks are legacy) | `relay_drivers/claude.rs` uses hooks, not ACP | P1 |
| ACP-2 | M21 | ACP driver `drive_step` unsupported — observe only | `acp.rs`: `drive_step` returns unsupported | P1 |
| ACP-3 | M21 | No agent capability negotiation (tools, modes, limits) | ACP protocol supports; not implemented | P2 |
| ACP-4 | M21 | No streaming response handling in relay | streaming not parsed in transcript or relay | P2 |
| ACP-5 | M21 | Permission/gate flow not implemented for ACP | only Claude hooks have gate logic | P2 |
| ACP-6 | M21 | Model selection via ACP not wired | ACP supports; not used | P3 |
| T-1 | T3 | tracker adapters (Jira/ADO) mockup-only; 2/7 gates buildable now, 2 scoped-read, 3 on #156 | reorg §9.2 | P1 |
| ~~T-2~~ | T2 | ~~join/membership flow dead~~ — resolved by #307 A3 invites (7e547a19) | plan gap analysis | — |
| F-3 | T1 | Dōjō sync works against the local dōjō; production unverified (needs an enrolled daemon); #184 open | production-readiness → `07-quality.md` | P1 |
| D-11 | P2 | website presents unbuilt as shipped (one claim already dropped) | site↔product audit | P2 |
| D-12 | P0(deps) | dependency drift: `@rokkit/*`, `@kavach/*` held back by lockfiles | production-readiness | P3 |
| D-13 | P0(deps) | 28 medium osv advisories (transitive; none code-fixable) | production-readiness | P3 |
| P-5 | P1 | Makefile/Homebrew macOS-only; Linux/Windows service + data-dir design first | reorg §10.3 | P1 |
| G-MEM-FEEDBACK | M9 | LLM-facing memory feedback tools (`report_memory_use`, `report_learning`) do not exist; feedback is HTTP-only (`…/reinforce`, `…/challenge`) | M9 status honesty; tool registry | P1 |
| G-MEM-SCOPE | M9 | `memory_scope` has no `user` / `org` rung; the cross-workspace and org-overlay ladder is designed, not built | M9 `memory_scope` enum vs doc ladder | P1 |
| G-PATTERN-STORE | M10 | `sensei.pattern_choices` / `sensei.pattern_registries` have no DDL; chosen options cannot persist | M10 status honesty; DDL inventory | P1 |
| G-TRACE-STORE | M11 | `sensei.trace_links` / `sensei.doc_lint_findings` have no DDL; bidirectional links and doc-doctor findings cannot persist | M11 status honesty; DDL inventory | P1 |
| G-TRACE-TOOL | M11 | `get_trace_links` is not a registered MCP tool | M11 status honesty; tool registry | P1 |
| G-GOV-AUDIT | M13 | `sensei.rule_violations`, `sensei.governance_overrides` have no DDL; only `rule_check_runs` ships; verifier gate + override store unbuilt | M13 status honesty; DDL inventory | P1 |
| G-LIB-WRAP | M12 | no `wrap_library` MCP tool; the wrap-me workflow has no write surface | M12 status honesty; tool registry | P2 |
| G-DOJO-AUDIT | M16 | no `/api/dojo-binding` endpoint; the About screen's Dōjō binding is opt-in only | M16 Surfaces; route inventory | P2 |
| G-IMPACT-TEST | M19 | impact/insights endpoints and tables (`applied_recommendations`, `impact_regressions`, `insights`) unbuilt | M19 scenarios all `NO TEST` | P2 |
| G-BILLING-DATA | T4 | billing tables (`billing_accounts`, `seats`, `subscriptions`, `active_contributors`, `invoices`) planned only | T4 status honesty; DDL inventory | P2 |
| G-COST-LEDGER | M5 | `inference_calls` table (cost/usage ledger) does not exist in DDL or code | M5 status honesty; DDL inventory | P2 |

## Remaining population (Phase 3)

The full register from [`../reorg.md`](../reorg.md) §12 spans the F- (faults),
P- (perf: e2e 23/130 red, `public.logs` 25 GB of a 37 GB DB, silent 1000% zoom
graphs), D- (doc conflicts and claims), T- (test coverage: no Rust tests in CI,
e2e flame-outs), M- (#153/#154/#155) and X- (backlog table) families. Each row
will name the module it blocks, so a plane file and this register always agree.

## How a gap closes

1. A `.feature` scenario in the module spec pins the behaviour (reorg §6).
2. The status flips in the plane file when there is evidence.
3. The row is removed from here and the work is tracked as a GitHub issue
   ([`../backlog.md`](../backlog.md)).