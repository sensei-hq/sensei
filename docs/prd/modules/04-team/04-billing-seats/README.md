# Billing & Seats · T4

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner))

## Purpose

Billing & Seats is the **business layer**: a hosted Dōjō can be
subscribed and metered. **Every dōjō gets the free tier** (D20). **A solo developer gets
everything free** on their personal dōjō, blocking targets included (D52). **An
organisation's public repositories are free**, targets included. **The paid tier is an
organisation's private repositories** — targets on them, integrations and consolidated
metrics in the dōjō — taken by its admin or owner, **per active contributor** (D46, D52,
narrowing D24). Sensei itself serves the individual; the shared views and targets are the
dōjō's.
The plan lives only in the dōjō — local sensei works in full on any plan. Read-only members are free;
GitHub Sponsors is optional support. There is **no separate Enterprise tier** —
SSO/SCIM and the audit trail are free on every dōjō (D59); extended audit retention and an SLA are options on the paid org tier. There is no
self-hosting (Q14). **Viewers and invited guests are never metered** — only active
contributors are.
The job is **meter what matters (active contributors), bill fairly, keep the
free tier real**. The user feels this as *I only pay when my team actually uses
the shared Dōjō, and the free tier isn't a trial.*

> **Status honesty.** The pricing model and tier table are designed (features/07-pricing.md).
> **Built:** the billing account and seat tables (`dojo.billing_accounts`,
> `dojo.seats`, the `dojo.tenant_seat_usage` count), GET/POST
> `/v1/t/[origin]/[org]/billing` (admin-only: the account plus live per-seat usage;
> POST refreshes the cached seat count), the org's **Plan & billing** section at
> `/org/[slug]/billing`, which reads that route, and seat opening on relay activity —
> `openOrRefreshSeat` (`dojo/src/lib/server/billing-data.ts:153`) inserts into `seats`
> and the relay session route calls it
> (`dojo/src/routes/v1/t/[origin]/[org]/relay/session/+server.ts:98`) when the update
> names a workspace. **Not built:** the daemon never sends `workspace_slug` — every
> call site passes `None` (`crates/senseid/src/dojo/client.rs:869,1033,1136`,
> `relay_project.rs:121`, `relay_run_project.rs:110`) — so no seat opens in practice;
> nothing calls `closeSeat` (`billing-data.ts:189`) when a member is removed; and there
> is no payment provider, invoices or plan changes. **There is no payment provider yet**: none is chosen and none is
> in the code, so nothing below that mentions invoices or payment exists. The mockup
> `dojo-billing.jsx` uses hardcoded example data.
> The website has a live GitHub Sponsors link. The free tier intent exists but
> the website says "free during preview" (not the final "free forever" claim).

## In scope / out of scope

- In: the three tiers (Free: a solo developer's personal dōjō and every public
  repository · Paid: an organisation's private repositories · GitHub Sponsors support,
  D52); **active contributor metering** (definition, detection,
  invoicing); plan selection in the organisation's Plan & billing (a dōjō arrives from a git
  integration on the free tier — D20); **Settings › Plan & billing for every
  organisation**, reading Free until a subscription exists (a personal dōjō has nothing
  to buy — D46) — `/org/[slug]/billing` (admin surface); the **no-Enterprise decision** (SSO/SCIM and audit
  free, D59; extended audit retention and SLA are paid org options).
- Out: the federation backbone (T1), role consoles (T2), workspace planning (T3),
  the marketing website claims (P2).

## Surfaces

- **App** — `/org/[slug]/billing` (admin: choose or change the plan, current plan, active contributor count, invoices, payment method,
  usage history, upgrade/downgrade).
- **Daemon** — **none yet** (metering/aggregation not built).
- **Worker API (built)** — `GET/POST /v1/t/[origin]/[org]/billing`: the account and
  live seat usage; POST recomputes `seats_used`. **Planned:** plan change, invoices,
  payment.
- **Data (built)** — `dojo.billing_accounts` (one per tenant: plan, status,
  seats_included, seats_used, `external_customer_ref` — NULL until a provider is
  chosen, period); `dojo.seats` (one row per user's active participation; a seat is
  billable on a private namespace); `dojo.tenant_seat_usage` (the live count).
  **Planned:** invoices and a provider subscription.

## Data invariants

- **Lead.** **Free = public repositories, forever** (D46). A public repository never
  requires a paid plan — an organisation's sync automatically (no member toggle; an
  admin may turn one off as org policy), a personal dōjō's, public and private, are the
  person's to choose, all by default.
- **Lead.** **A personal dōjō is free, private repositories included** (D46).
- **Lead.** **An organisation's private repositories need a subscription**, taken by
  its admin or owner. Until there is one they are **hidden** from the organisation's
  repository lists, which say private repositories are
  supported on paid plans and link to Plan & billing (D46).
- **Lead.** **The plan lives only in the dōjō.** Local sensei (app, daemon, CLI, MCP)
  works in full on any plan; a subscription is needed only for sync and the dōjō's
  benefits (D46).
- **Lead.** **What the paid tier buys** (D52): targets on an organisation's private
  repositories, integrations, and consolidated metrics in the dōjō. An org's public
  repositories get targets free; a personal dōjō gets everything free.
- **Lead.** **Paid = per active contributor.** Read-only members are free.
  "Active contributor" = a user who authored a commit/push/contribution in the
  billing window (exact definition TBD — gap).
- **Lead.** **No separate Enterprise tier.** SSO/SCIM, audit retention, SLA are
  options on the paid org tier.
- **Lead.** **Viewers and invited guests are never metered** (T6) — a seat is an active
  contributor.
- **Lead.** **Every dōjō starts on the free tier**; the plan is chosen in the
  organisation's Plan & billing, never at creation — a dōjō arrives from a git integration (D20).
- **Lead.** The billing portal is **admin-only** in the org context.

## Signals shown

| Element | Meaning | Worked example | Decision it drives / next step |
|---|---|---|---|
| Plan badge | Free / Paid / Sponsor | `Paid · 12 active contributors` | Free on an org → subscribe to sync its private repositories (D46, D52) |
| Active contributor count | users who contributed in the window | `12 / 15 seats used` | near the included seats → expect a higher bill, or review who is active |
| Billing window | monthly / annual | `Monthly · renews 2026-11-13` | none — candidate to cut |
| Invoice list | past invoices with status | `INV-2026-10 · $240 · paid` | none — candidate to cut until a payment provider exists |
| Payment method | card / invoice / sponsored | `Card ending ·••• 4242` | none — candidate to cut until a payment provider exists |
| Usage history | contributor count over time | chart: 8 → 12 → 11 → 12 | none — candidate to cut |

## Done gate

- A dōjō imported from a git integration starts on Free; an admin chooses Paid in
  the billing page, and the plan is recorded on the tenant.
- The billing portal (`/org/[slug]/billing`) shows the current plan, active
  contributor count, invoices, and payment method (admin only).
- Active contributors are metered correctly (definition implemented); once a payment
  provider is chosen (none yet), invoices are generated and paid through it.
- Downgrade/upgrade works; the free tier is truly free for public repositories.
- An organisation's private repositories stay hidden until its admin or owner
  subscribes; a personal dōjō syncs its private repositories free (D46).
- GitHub Sponsors link is present and functional.

```bash
# Built: the account and live seat usage (admin only); no provider, no invoices
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/billing" | jq
```

## Wrong gate

- **A public/OSS repo requires a paid plan** — free tier regressed.
- **An organisation's private repository is listed or synced with no subscription**
  — the D46 gate regressed.
- **A personal dōjō asks for a subscription, or hides its private repositories** —
  D46 regressed.
- **Local sensei refuses a feature because of the plan** — the plan leaked out of
  the dōjō (D46).
- **A read-only member is counted as an active contributor** — metering
  definition regressed.
- **An invoice is generated for a free-tier Dōjō** — billing logic regressed.
- **The billing portal is visible to non-admins** — role-gating regressed.
- **The Enterprise tier appears as a separate plan** — tier model regressed.

## Scenarios

- [`scenarios/billing-seats.feature`](scenarios/billing-seats.feature) — the
  tier model, active contributor metering, plan selection, billing portal
  (all `NO TEST` — the account, seats and billing route are built but not pinned here).

## Open gaps

- **Active contributor definition** — how is "active" defined (seat metering)?
- **Fair-use limits, storage caps, non-profit/education pricing?**
- **Payment processor integration** — none selected (`external_customer_ref` stays NULL).
- **Metering aggregation** — daemon-side not built.

## Related

- [features/07-pricing.md](../../../../features/07-pricing.md) — pricing
  feature doc (mock tier table, flows, status table).
- [design/dojo-web.md](../../../../design/dojo-web.md) — org context billing
  surface (admin).
- [mockups/Sensei/lib/dojo/dojo-billing.jsx](../../../../mockups/Sensei/lib/dojo/dojo-billing.jsx)
  — visual source (tier table + per-seat example, hardcoded data).
- [mockups/Sensei/lib/dojo/dojo-saas.jsx](../../../../mockups/Sensei/lib/dojo/dojo-saas.jsx)
  — SaaS entry: sign-in · orgs · create Dōjō (plan choice). Superseded: a dōjō comes
  from a git integration on the free tier and the plan is chosen in billing (D20).
- [prd/01-vision.md §Business model](../../../01-vision.md) · [prd/03-personas.md §4,5](../../../03-personas.md).
- Crates: **none yet** — Worker: `dojo/src/routes/v1/t/[origin]/[org]/billing/+server.ts`;
  the org section is `/org/[slug]/billing`, served by `dojo/src/routes/(app)/org/[slug]/[section]/`.