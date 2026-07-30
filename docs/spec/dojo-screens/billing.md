# Plan & billing — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `(app)/org/[slug]/[section]` where `section = billing` → `ScrBilling` (`+page.svelte` L142, the `{:else}` default branch after all named sections).
- Mockup: dojo2-app.jsx `ScrBilling` (L928)
- Access axis: tenant-primary — org-financial data, admin-gated; billing is per-tenant (`docs/architecture/entity-access-model.md` §3, org console → `tenant_id`). Read route is ADMIN-floor.
- Status: PARTIAL — the **live billable seat count** (`dojo.tenant_seat_usage` model) and, when a `billing_accounts` row exists, the **plan + renewal date** are real; the price-per-seat, the three pricing tiers, the relay free/paid rows, and the invoice history are all illustrative fixtures (D-BILLING = schema + route only; no payment provider wired).

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
`toKitBilling` (`dojo/src/lib/billing-map.ts:29`) overlays only `seatsActive`/`plan`/`renews`, so **`invoices`, per-seat price, `tiers`, `relayRows`, and `seatsReadonly` stay fixture even on a successful 200**; `(app)/org/[slug]/[section]/+page.ts:205` (`guardedFor('billing', billingFor(slug), …)`) falls back to the FULL fixture catalog on error; `billingError` (`+page.ts:252`) is computed but never surfaced; `ScrBilling.svelte:147` renders fake paid invoices (`$408`/`$396`/`$372` from `fixtures.ts:1326`). **Impact:** an org admin always sees fabricated "paid" invoices + fake per-seat pricing/tiers, and a billing-read outage shows an all-fake screen with no error — money-facing. **Fix on build:** drive every field from the real `/v1` read; on a fetch error render an explicit error state — NEVER the fixture; honest-empty only when genuinely empty. (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Header plan chip | `b.plan` | `billing-map.ts toKitBilling` → `planLabel(billing_accounts.plan)` when account exists, else fixture `'Team · private'` | have (when account) / plumb | no |
| Current-plan card · plan | `b.plan` | as above (`dojo.billing_accounts.plan`) | have/plumb | no |
| Current-plan card · renews | `b.renews` | `shortDate(billing_accounts.period_end)` when set, else fixture `'Aug 1'` | have/plumb | no |
| Current-plan card · $/seat | `b.perSeat` | **FIXTURE** (`fixtures.ts billing.perSeat`; no pricing-catalog table) | plumb | no |
| Billable seats · count | `b.seatsActive` | `res.usage.seats_used` — **LIVE**: `server/billing-data.ts summarizeSeatUsage(loadActiveSeatRows)` = unique users with an active `dojo.seats` row on a **private** `sensei.namespaces` project (deduped) | have | no |
| Billable seats · read-only sub | `b.seatsReadonly` | **FIXTURE** | plumb | no |
| This-month card · total | `monthly = seatsActive × perSeat` | `billing-view.ts monthlyTotal` — live seats × **fixture** perSeat (half-real; "updated live" label is load-time, not push) | have/plumb | no |
| Tier cards (Free/Team/Enterprise) | `b.tiers[]` | **FIXTURE** (`KitBillingTier[]`; kanji/price/lines/current/dark). No pricing-catalog source | plumb | no |
| Tier CTA (Downgrade/Contact sales) | `tierCtaLabel(t.id)` | label only — inert (no billing portal) | plumb | — |
| Relay free-vs-paid rows | `b.relayRows[]` | **FIXTURE** (`relayTone` styles the free/paid chip) | plumb | no |
| Invoices list | `b.invoices[]` | **FIXTURE** (date/amount/status; no invoice table, no payment provider) | plumb | no |

## APIs / loaders
- **Loader:** `dojo/src/routes/(app)/org/[slug]/[section]/+page.ts`, `guardedFor('billing', billingFor(slug), getBilling, res => toKitBilling(billingFor(slug), res))`. The fixture (`billingFor` = the illustrative catalog) is the base; the live GET overlays only `seatsActive` (+ plan/renews when an account exists). Degrades to the pure fixture + `billingError` on failure.
- **Read route:** `dojo/src/routes/v1/t/[origin]/[org]/billing/+server.ts` (GET, ADMIN) → `server/billing-data.ts`: `summarizeSeatUsage(loadActiveSeatRows(tenantId))` + `getBillingAccount(tenantId)`. Returns `{ account, usage }`.
  - `loadActiveSeatRows` — `dojo.seats` (active, `ended_at IS NULL`) × `sensei.namespaces` (visibility/name/slug), cross-schema two-query join; only `visibility='private'` seats count; orphan seats dropped.
  - `usage.billable_users[]` — the per-user private-project breakdown (the defensible count) is returned but **not surfaced** on the screen.
- **Recompute route:** same `billing/+server.ts` POST → `refreshSeatsUsed` upserts `dojo.billing_accounts.seats_used` + `seats_computed_at`. Not called from this screen.
- **Client:** `admin-data.ts getBilling` (types `BillingResponse`/`BillingAccount`/`BillingUsage`). **Mapper:** `billing-map.ts`. **View helpers:** `billing-view.ts` (`monthlyTotal`, `relayTone`, `tierCtaLabel`).

## Interactions & states
- **Degraded** — read failure → pure fixture catalog + `billingError`. Screen always renders.
- **Non-admin** — 403 → guard degrades to fixture; renders.
- **Mobile** — 3-col grids collapse to 1-col (`md:grid-cols-3`). Wired.
- **Tier / invoice CTAs** — inert (no billing portal / payment provider).
- **Live** — `seatsActive` is fresh per load; "updated live" is a label, not a subscription.

## Gap / to-do (vs mockup)
1. **Pricing is fixture.** `perSeat`, `tiers`, `relayRows` have no table — the whole revenue surface except the seat count is illustrative. A pricing catalog (or hardcoded product config) + a payment provider is D-BILLING follow-on.
2. **Invoices are fixture** — no invoice table, no Stripe/provider. The list is decorative until billing is wired.
3. **`monthlyTotal` mixes live × fixture** — the dollar figure is not authoritative (live seats, fixture rate). Flag as illustrative until `perSeat` is real.
4. **Seat breakdown unused** — `usage.billable_users[]` (who consumes each seat + which private projects) is the defensible detail; the mockup has no drill-down. Consider surfacing it (per the admin console's "count is defensible" intent).
5. **Recompute not exposed** — the POST recompute path exists but no UI triggers it; seat count is whatever the GET computes at load (already live, so low priority).

## Open questions (for Jerry)
- Where does the pricing catalog live — a `dojo` table, a Worker constant, or the marketing site's source of truth? Until decided, tiers/perSeat/relayRows stay fixture.
- Payment provider (Stripe?) — is invoice history in scope for this pass, or does the screen stay "seat count + plan is real, pricing is illustrative" until a provider is wired?

### Resolved design (2026-07-30)
- **Q1 pricing catalog → a versioned Worker PRODUCT CONFIG/constant** (the real price list — `tiers`, `perSeat`, `relayRows` read from it). NOT a DB table, NOT fabricated; keep in sync with the marketing site.
- **Q2 payment provider → DEFER (no Stripe this pass).** REAL: live seat count (`dojo.tenant_seat_usage`) + plan/renewal (`dojo.billing_accounts`) + catalog pricing (from Q1). **Invoices → honest-empty** — DROP the fabricated `$408/$396/$372` fixture invoices (no invoice table / provider yet). Billing/portal CTAs inert until a provider.
- **Build constraint (money-facing fabrication debt):** drive every field from the real read; on a billing-read error render an explicit ERROR state — NEVER the fixture catalog; honest-empty invoices, never fabricated "paid" rows.
- **Depends on:** the Worker pricing config + existing ADMIN `GET …/billing` (seat usage + `billing_accounts`) + `refreshSeatsUsed` POST (exists). Provider/invoices = deferred follow-on.
- Should the seat card link to the per-user `billable_users` breakdown (defensible count), or is the aggregate enough for the admin view?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type Billing = { plan: string; renews: string | null; perSeat: Money | null;
  seats: SeatUsage; tiers: BillingTier[]; relayRows: RelayRow[]; invoices: Invoice[] }
type SeatUsage = { active: number; readonly: number | null; billableUsers: BillableUser[] }
type BillingTier = { id: string; kanji: string; price: string; lines: string[]; current: boolean }
type RelayRow = { label: string; free: string; paid: string }
type Invoice = { id: string; date: string; amount: string; status: string }
```
`seats.active` + `billableUsers` are the **real, defensible** parts (live `dojo.seats × sensei.namespaces` private-project count). `perSeat`, `tiers`, `relayRows`, `invoices` are `null`/empty-tolerant on purpose — no pricing catalog + no payment provider, so Load returns the illustrative fixture and the UI flags it rather than fabricating authoritative revenue.

**State** — `billing-state.svelte.ts` → `billingState`
- data: `billing: Billing | null`
- `$derived`: `monthlyTotal` (`seats.active × perSeat`, **null/illustrative when `perSeat==null`**), `hasAccount`, `seatBreakdown` (`billableUsers`)
- methods: `load(billing)`, `recompute()` (POST `refreshSeatsUsed`), `openPortal(tierId)` (inert until a provider)

**Load** — `billing.ts` → `loadBilling(tenantKey, slug)`
- mock-first: the fixture catalog (tiers / relay / invoices / perSeat) + mock `SeatUsage` — matches the illustrative `ScrBilling`
- real (later, body-swap only): existing ADMIN GET `…/billing` (see APIs above) → `summarizeSeatUsage(loadActiveSeatRows)` (unique users on active private-project seats) + `getBillingAccount` (`dojo.billing_accounts.plan`/`period_end`) overlaying live `seats.active` + plan/renews onto the fixture. `perSeat`/`tiers`/`relayRows`/`invoices` stay fixture until a **pricing catalog** + **payment provider** exist (WS-3). Recompute POST already exists.

**Components** (pure, semantic, own styles + `md:` — no `K2*`)
- `BillingConsole` shell — 3-col grids collapsing to 1-col (`md:`)
- `PlanCard` — plan · renews · $/seat (perSeat flagged illustrative)
- `SeatCard` — billable seats count + read-only sub; optional drill → `SeatBreakdown` (`billableUsers`, the defensible detail)
- `MonthlyCard` — this-month total (illustrative until `perSeat` real; "updated live" is a load-time label)
- `TierGrid` + `TierCard` — Free/Team/Enterprise (fixture); CTA inert until a portal
- `RelayComparison` — free/paid rows (fixture)
- `InvoiceList` — invoices (fixture/empty until a provider)
- Kanji tier marks = `KanjiToken` (brand); glyphs = Solar icons

**Copy** (paraglide `m.<key>()`): plan/seat/tier/invoice labels, the "illustrative until billing is wired" copy, the "updated live" seat label. No inline literals.

**Realtime = State**: none — `seats.active` is fresh per load; "updated live" is a label, not a subscription. **Test seams:** state methods (`monthlyTotal` null-when-`perSeat`-null; `hasAccount`); `PlanCard`/`SeatCard`/`TierCard` with mock props (incl. no-account path); Load mock → shape. Billing is tenant-primary (correct) — no Rule A/B/C touch.
