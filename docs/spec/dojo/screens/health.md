# Health — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/health` — served by `(app)/org/[slug]/[section]` (`section === 'health'`); read API `GET /v1/t/[origin]/[org]/health`.
- Mockup: dojo2-app.jsx `ScrHealth` (L1306)
- Access axis: **tenant-primary** — org-console admin surface. Canonical `docs/architecture/entity-access-model.md` §3: "Org console (`/org/[slug]`: … audit … team runs) → **Tenant** → `tenant_id`". The rollup filters every count by `tenant_id` at the ADMIN role floor.
- Status: **PARTIAL** — the 4 signal cards are REAL (a live rollup over `dojo.relay_sessions` / `triage_queue` / `audit_events` / `memberships`), but they are RELABELED from the mockup's four; the contributions-vs-approvals bar chart and the leak-guard/anomaly alert list render **EMPTY** (`toKitHealth` returns `[]` for both — the richer read isn't built).

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Banner (観) copy | literal | literal in component | have | no |
| Signal card 1 | `Sessions this week` `312` `↑14%` | **relabeled →** "Live connections" = `HealthRollup.connections` = count of `dojo.relay_sessions` with `heartbeat_at ≥ now-5min` (sub: "last 5 min") | bind | no (poll-able) |
| Signal card 2 | `Adoption rate` `68%` | **relabeled →** "Queue depth" = `queue_depth` = `dojo.triage_queue` where `state='queued'` (sub: "awaiting triage") | bind | no |
| Signal card 3 | `Leak-guard blocks` `3` | **relabeled →** "Published · 1h" = `publish_rate_1h` = `dojo.audit_events` where `action ∈ {publish,approve,distribute}` and `ts ≥ now-1h` | bind | no |
| Signal card 4 | `Queue age · median` `6h` | **relabeled →** "Sync errors" = `error_rate_1h` = `dojo.memberships` where `sync_status='error'` (tone warns if >0) | bind | no |
| Card kanji/tone | `s.kanji` / `s.tone` | `toKitHealth` sets fixed kanji (観/門/承/盾) + tone per card | have | no |
| Contrib-vs-approve chart | `h.contribVsApprove[]` `{wk,c,a}` | **empty `[]`** from `toKitHealth` — no weekly time-series read exists | plumb | no |
| Chart bar geometry | `x.c/max`, `x.a/max` | `$lib/health-view.ts` `barMax`/`barPct` (pure) — correct but fed an empty series | have | no |
| Leak-guard & anomalies list | `h.alerts[]` | **empty `[]`** from `toKitHealth` — no anomaly/alert read exists | plumb | no |
| Alert count | `h.alerts.length` | 0 (empty) | plumb | no |

## APIs / loaders
- **Loader** `(app)/org/[slug]/[section]/+page.ts` L195–200: `guardedFor('health', toKitHealth({connections:0,queue_depth:0,publish_rate_1h:0,error_rate_1h:0}), getHealth, toKitHealth)`. Guarded on `tenantKey = org.url`; degrades to a zeroed rollup + `healthError` on failure/403/dev-404.
- **Client** `$lib/admin-data.ts::getHealth()` → `GET /v1/t/{tenant}/health` → bare `HealthRollup`.
- **Endpoint** `routes/v1/t/[origin]/[org]/health/+server.ts` — `resolveTenantAccess(..., ACCESS.admin)` then `$lib/server/admin-data.ts::getHealth(db, tenantId)`: four isolated `count/head` queries (one missing table can't blank the whole strip; a query error throws — never a silent 0).
- **Mapper** `$lib/admin-map.ts::toKitHealth(rollup)` — 4 signal cards; `contribVsApprove: []`, `alerts: []` (explicitly noted "a richer read").

## Interactions & states
- Presentational; `max = $derived(barMax(contribVsApprove))` — with an empty series the chart area renders no bars (graceful, not broken).
- No realtime today — the rollup is a load-time snapshot. `connections` is heartbeat-derived and would be the natural realtime candidate (relay already has a realtime channel, `$lib/relay-realtime.ts`), but Health does not subscribe.
- Errors: `healthError` surfaced by the loader; the zeroed fallback keeps the strip rendering.

## Gap / to-do (vs mockup)
1. **Contributions-vs-approvals series.** Build a weekly time-series (last N weeks of `dojo.artifacts` contributions vs `dojo.decisions` approvals, or `audit_events` grouped by week) → populate `contribVsApprove`. Today it is hardcoded empty.
2. **Leak-guard & anomaly alerts.** No source: needs a real signal — dereference/leak-guard containment events (per the confidentiality invariant) + anomaly detection (e.g. an unowned scope queue, sync errors). The mockup's alerts (held stack trace, ownerless Postgres queue) imply reads the tenant audit trail doesn't yet distinguish. `admin-data.ts::getHealth` already notes error audit events aren't separated from the sync-error proxy — same follow-up.
3. **Signal parity (decide: adopt the real four or restore the mockup four).** The mockup shows Sessions-this-week / Adoption-rate / Leak-guard-blocks / Queue-age-median; the real rollup shows Live-connections / Queue-depth / Published-1h / Sync-errors. Adoption rate and median queue age need reads that don't exist; leak-guard blocks need the containment-event source from (2).
4. Consider realtime for `connections` (reuse the relay realtime channel) so the strip is live, not a snapshot.

## Open questions (for Jerry)
- Keep the real four signals (connections / queue depth / published-1h / sync errors) as the shipped set, or invest in the mockup's four (sessions, adoption rate, leak-guard blocks, median queue age)? The latter two need new reads.
- What is the canonical **leak-guard / containment** event source for the alert feed — a `dojo.audit_events.action` value, a dedicated incidents/containment table, or the confidentiality ledger? This blocks both the alert list and the "Leak-guard blocks" signal.

### Resolved design (2026-07-30)
- **Q1 signals → BUILD the mockup's four:** sessions · adoption-rate · leak-guard-blocks · median-queue-age. Needs new reads — `sessions` (from `dojo.relay_sessions`), `adoption-rate` + `median-queue-age` (new rollups), `leak-guard-blocks` (from the containment source, Q2). Replaces the interim relabeled real-four.
- **Q2 alert source → `dojo.audit_events` containment events.** The always-on dereference HOLD path (contribute.rs held) emits hold/containment events to `dojo.audit_events` with a defined `action` (`'contained'`/`'held'`); the alert feed AND the leak-guard-blocks signal read them. (The confidentiality-ledger option is retired — the strip-gate was removed in Rule B.)
- **Also build:** the contrib-vs-approve weekly chart = a new time-series read (`dojo.artifacts` vs decisions / `audit_events` by ISO week). Renders no bars gracefully until then.
- **Depends on:** new health reads (adoption-rate, median-queue-age, weekly series) + emitting containment events to `dojo.audit_events` + the alert-feed read.
- Contrib-vs-approve: bucket by ISO week over `dojo.artifacts` + `dojo.decisions`, or over `audit_events`? Over what window (mockup shows 4 weeks)?
- Is a live/realtime Health strip in scope, or is a load-time snapshot fine for v1?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** (reference §Elements→data): the 4 signal cards = a live rollup over `dojo.relay_sessions` (heartbeat) / `dojo.triage_queue` (queued) / `dojo.audit_events` (publish/approve, 1h) / `dojo.memberships` (sync_status). The contrib-vs-approve weekly series (`dojo.artifacts` vs `dojo.decisions`, or `audit_events` by ISO week) and the leak-guard/anomaly feed have **no read yet**.

**API** (reference §APIs/loaders): `GET /v1/t/{tenant}/health` (ADMIN floor) → `HealthRollup` (4 real counts; `contribVsApprove:[]`, `alerts:[]`). Series + alert endpoints don't exist (the leak-guard/containment source is an open Q).

**Domain types** (UI-shaped):
```ts
type HealthDashboard = { signals: Signal[]; contribVsApprove: WeekBar[]; alerts: Alert[] }
type Signal = { id; kanji; label; value; sub; tone: 'ok'|'warn'|'accent'; delta?: string }
type WeekBar = { week: string; contributions: number; approvals: number }
type Alert = { id; kind; title; detail; ts; severity: 'info'|'warn'|'critical' }
```

**State** — `health-state.svelte.ts` → `healthState`
- data: `dashboard: HealthDashboard`
- `$derived`: `barMax` (from `contribVsApprove`; reuse `health-view.ts` `barMax`/`barPct`)
- methods: `load(dashboard)`, `patch(signal)` (realtime connections), `subscribe()`

**Load** — `health.ts` → `loadHealth()`
- mock-first: hand-crafted `HealthDashboard` — 4 signals + a 4-week `WeekBar[]` + a couple alerts
  (mirror the mockup) so the chart + feed **build to fidelity NOW** (today they render empty)
- real (body-swap only): `getHealth` → `toKitHealth` for the 4 signals; the series + alerts stay
  empty until their reads exist (chart renders no bars gracefully via `barMax`)

**Components** (pure, semantic, own styles + `md:`; NO `K2*`)
- `HealthDashboard` shell — banner (観 `KanjiToken`) + `HealthSignalStrip` + `ContribApproveChart` +
  `AlertFeed`.
- `SignalCard` — kanji/glyph · label · value · sub · tone (warns when >0 sync errors). Solar icon for
  the functional glyph; the card kanji is a `KanjiToken` brand mark. **Mockup-match + `md:` here.**
- `ContribApproveChart` — bars from `WeekBar[]`, geometry via `health-view.ts` `barMax`/`barPct` (pure).
- `AlertFeed` / `AlertRow` — leak-guard + anomaly items; EmptyState when none.
- Shell reuses `(app)/org/[slug]/[section]`; `+page.ts` = Load wiring → `healthState.load`.

**Copy** (paraglide `m.<key>()`): card labels/subs, chart axis, alert / empty copy in
`messages/en.json`; 観 stays a `KanjiToken`, functional glyphs are **Solar icons**.

**Realtime = State**: `connections` is heartbeat-derived → the natural subscribe candidate;
`subscribe()` reuses the relay realtime channel (`relay-realtime.ts`) and `patch`es the signal
(targeted, not `invalidateAll`). Snapshot-only is acceptable for v1 (open Q). **Test seams:** state
methods + `barMax` (no DOM); `SignalCard`/`ContribApproveChart`/`AlertRow` with mock props; Load mock
→ shape.
