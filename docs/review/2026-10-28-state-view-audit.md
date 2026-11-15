# Audit — 2026-10-28: where views decide what state should own

Asked by the user after the identity bug ("credential expired" beside "ready": a daemon `action`, a view-derived
sentence and a screen-local clock, combined differently on two screens). The rule (app/CLAUDE.md §2): `*.svelte.ts`
state owns derivations, status, copy, thresholds and actions; components render. Reference for "right":
`PersonaList.identities` (app/src/lib/personas.svelte.ts). Read-only audit of `app/src` and `dojo/src`; line numbers
±2. Ranked by the risk of two screens telling the user contradictory things.

| # | Finding | Where | Risk | Owner |
|---|---|---|---|---|
| 1 | Memory strength (1–5 in the DB) on three scales | workspace memories `+page.svelte:168,183` (×100 → "300%"); insights `MemoryRow.svelte:15` + `insights-board.svelte.ts:150` (clamped 0–1 → always full); learnings `MemoryDetail.svelte:43`, `ActiveList.svelte:25`, `MemoryList.svelte:36,73,132` ("3.0 / 5") | one memory reads 300%, full and 3.0/5. Verified: live strengths 1–3 | one `strengthView()` in a memory state module |
| 2 | Settings · Metrics picks its write identity itself | `settings/metrics/+page.svelte:29`, `signedInSlot` (`metric-status-state.svelte.ts:226`) — first slot, ignores status and default, read once | writes with a dead or non-default credential while the overlay says expired. Verified | `PersonaList` — **fixed with the identity work** |
| 3 | Two stores decide credential standing; the layout picks whose error shows | `forge-auth.svelte.ts:58` (own 5-min poll) vs `PersonaList` (60 s); `+layout.svelte` `personaList.error ?? forgeAuth.lastError` | renewal error vs "ready" list | fold renewal standing into `PersonaList` |
| 4 | Overlay vs Settings · Dōjō leftovers | no loading phase in settings; "ready" badge in one view only; `githubLogin ?? label` vs `label`; overlay footer says "separate window" (contradicts #300); `start/stop` on the shared singleton from a page | the two identity views still differ | `PersonaList` — **fixed with the identity work** |
| 5 | Dōjō relay run/step status mapped three ways; progress counted three ways | `RelayCard.svelte:13-58`, `relay-view.ts:23-56`, `kit/InboxRow.svelte:13-27`; `RelayDetail.svelte:33`, `PlanOutline.svelte:29`, `PlanPips.svelte:14` | red in the list, "Needs you" in the detail | `relay-view.ts` |
| 6 | FTR bands and the session FTR tone scattered | `WorkspaceDot.svelte:8`, `workspace-sidebar-view.ts:67`, workspace instruments `+page.svelte:167` (<0.5), `buckets.ts:118`, `RecentSessions.svelte:57` (unscored → warning), observatory instruments `+page.svelte:572` | the same FTR banded differently per screen | `lib/ftr.ts` |
| 7 | Corrections phrase worded four ways ("1 corrections") | `RecentSessions.svelte:15`, `SessionRow.svelte:34`, `overview-view.svelte.ts:170`, `metric-view.ts:498` | | `metric-view.ts` `correctionsPhrase` |
| 8 | Tool-health thresholds duplicated; two "total calls" one click apart | observatory instruments `+page.svelte:929,835`, `state/mcp.svelte.ts:42`, workspace instruments `:162`, `ToolHealthCard.svelte:71` | "healthy" chip beside a red rate | `state/mcp.svelte.ts`, `instruments-view.ts` |
| 9 | FTR-delta tone from the raw sign, label rounded ("±0%" in green) | impact pages (`observatory/impact:75,135`, workspace impact `:66,119`), patterns `+page.svelte:21` (own `formatDeltaPct`) | | `lib/impact.ts` `deltaView()` |
| 10 | Traceability drift tone decided twice (unknown → green vs muted) | workspace traceability `+page.svelte:10`, `traceability-state.svelte.ts:71` | | shared traceability state |
| 11 | Health status → tone mapped twice on one row | `StatusIndicator.svelte:16`, `GateRow.svelte:11` | | `health-state.svelte.ts` |
| 12 | Assistant "configured" defined twice | `AssistantCard.svelte:42-82`, `setup/done/+page.svelte:6`, `setup/assistants/+page.svelte:61` (own SSE) | header "off" while configuring | `wizard-state.svelte.ts` |
| 13 | Writes and state machines in components, duplicating the wizard | `InferenceAssignmentsPanel.svelte`, `ProvidersSection.svelte:28` (= `wizard-state.svelte.ts:243`), `upgrade/+page.svelte:17-115`, dōjō `RelayNotifyToggle.svelte` | | wizard / new upgrade + relay-push state |
| 14 | A clock in a component | `activity-logs/+page.svelte:44` (`now` read once) | "running", "next in …" go stale | `scheduled-tasks.svelte.ts` |
| 15 | Copy and formatting assembled in components (low) | setup done, `RecentSessions` time formats, three ms formatters, local tone ternaries (upgrades, metrics, insights), `TrendChart`, dōjō `ScrSharing`, setup scan | | the screens' state files |

Clean: workspace overview, about, integrations, tickets, metrics list, health logs, sessions digest, consolidation,
libraries, dōjō connections and sharing, share review, every `+page.ts` loader, most dōjō org and you screens.

## Status — 2026-10-28

| # | Done in | |
|---|---|---|
| 2, 4 | b60ea672 | identity write slot, phase, name, badge; ref-counted clock |
| 3 | 09703055 | renewal joined the identity state; ForgeAuth removed |
| 1 | 385ab7f8 | strengthView on the 1–5 scale; MemoryList (dead) removed |
| 5 | 9abfbbc1 | relay step colours from the kit's K2_NODE; run table per the DB's meaning; sessionView, runProgress |
| 6 | be7a787b | lib/ftr bands (0.8 / 0.6, neutral middle) and sessionFtrView |
| 7 | 137bfab6 | correctionsView |
| 8 | f031c884 | toolHealthView, toolUsageTotals, toolSourceView, toolKindView, callVerdictView |
| 9 | 0c9bd0b4 | deltaView (tone from the rounded value), verdictToneVar |
| 10 | 0b883bf2 | lib/traceability: driftDotStatus, DriftScan |
| 11 | c3be0afd | componentStatusView |
| 12 | 731a2821 | assistantViewOf / assistantView, configuredAssistants, watchAssistantEvents |
| 13 | 7f885dde, 1b5255c6, cbf6dfde, 5206abfb | saveRouterKey; ChainRoles + assignChainRoles; UpgradeRun (a run that cannot start is a failure; no simulated progress); RelayNotify — "Notify me" in the relay inbox header, for every dōjō the inbox spans |
| 14 | 7b7fe532 | lib/clock — a ticking, ref-counted clock |
| 15 | 241a1c2f, 4a7db153, fcda410f, 1df91a79, cd7a7daa4 | lib/format; scan progressOf; one session-duration format (compactDuration, days); doneSummary; KIND_META tone, reasonKindClass, columnToneClass; grainUnit, dayTone, rollingFtr; dōjō sharing labels. All 15 done. |

