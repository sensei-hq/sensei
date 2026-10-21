# Billing & Seats · T4

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard))

## Purpose

Billing & Seats is the **business layer**: a private shared Dōjō can be
subscribed and metered. **Every dōjō gets the free tier** (D20); billing pays for
what is beyond it — **gating, integrations and consolidated metrics in the dōjō** —
**per active contributor**. Sensei itself serves the individual; the shared views and
gates are the dōjō's. The model stays **free for individuals and open source**; read-only members are free;
GitHub Sponsors is optional support. There is **no separate Enterprise tier** —
SSO/SCIM, audit retention, SLA become options on the paid org tier. There is no
self-hosting (Q14). **Viewers and invited guests are never metered** — only active
contributors are.
The job is **meter what matters (active contributors), bill fairly, keep the
free tier real**. The user feels this as *I only pay when my team actually uses
the shared Dōjō, and the free tier isn't a trial.*

> **Status honesty.** The pricing model and tier table are designed (features/07-pricing.md).
> **Nothing is built** — no billing integration, no metering, no invoices, no
> payment processor. The mockup `dojo-billing.jsx` uses hardcoded example data.
> The website has a live GitHub Sponsors link. The free tier intent exists but
> the website says "free during preview" (not the final "free forever" claim).

## In scope / out of scope

- In: the three tiers (Free individuals/OSS · Paid organizations · GitHub
  Sponsors support); **active contributor metering** (definition, detection,
  invoicing); plan selection in the org's billing page (a dōjō arrives from a git
  integration on the free tier — D20); billing portal in org context
  (`/org/[slug]/billing` — admin surface); the **no-Enterprise decision** (fold
  SSO/SCIM, audit retention, SLA into paid org options).
- Out: the federation backbone (T1), role consoles (T2), workspace planning (T3),
  the marketing website claims (P2).

## Surfaces

- **App** — `/org/[slug]/billing` (admin: choose or change the plan, current plan, active contributor count, invoices, payment method,
  usage history, upgrade/downgrade).
- **Daemon** — **none yet** (metering/aggregation not built).
- **Worker API (planned)** — `/v1/t/{tenant}/billing/*` (plan, metering,
  invoices, payment).
- **Data (planned)** — `dojo.tenants` (plan, status, billing_anchor);
  `dojo.subscriptions` (stripe_customer_id, price_id, status);
  `dojo.active_contributors` (tenant, user, window_start, window_end, count);
  `dojo.invoices` (tenant, stripe_invoice_id, amount, period, status).

## Data invariants

- **Lead.** **Free = individuals & open source, forever.** Public/OSS repos
  never require a paid plan.
- **Lead.** **Paid = per active contributor.** Read-only members are free.
  "Active contributor" = a user who authored a commit/push/contribution in the
  billing window (exact definition TBD — gap).
- **Lead.** **No separate Enterprise tier.** SSO/SCIM, audit retention, SLA are
  options on the paid org tier.
- **Lead.** **Viewers and invited guests are never metered** (T6) — a seat is an active
  contributor.
- **Lead.** **Every dōjō starts on the free tier**; the plan is chosen in the org's
  billing page, never at creation — a dōjō arrives from a git integration (D20).
- **Lead.** The billing portal is **admin-only** in the org context.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Plan badge | Free / Paid / Sponsor | `Paid · 12 active contributors` |
| Active contributor count | users who contributed in the window | `12 / 15 seats used` |
| Billing window | monthly / annual | `Monthly · renews 2026-11-13` |
| Invoice list | past invoices with status | `INV-2026-10 · $240 · paid` |
| Payment method | card / invoice / sponsored | `Card ending ·••• 4242` |
| Usage history | contributor count over time | chart: 8 → 12 → 11 → 12 |

## Done gate

- A dōjō imported from a git integration starts on Free; an admin chooses Paid in
  the billing page, and the plan is recorded on the tenant.
- The billing portal (`/org/[slug]/billing`) shows the current plan, active
  contributor count, invoices, and payment method (admin only).
- Active contributors are metered correctly (definition implemented); invoices
  are generated and paid via the payment processor.
- Downgrade/upgrade works; the free tier is truly free for individuals/OSS.
- GitHub Sponsors link is present and functional.

```bash
# Not implemented — placeholder
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/billing" | jq
```

## Wrong gate

- **A public/OSS repo requires a paid plan** — free tier regressed.
- **A read-only member is counted as an active contributor** — metering
  definition regressed.
- **An invoice is generated for a free-tier Dōjō** — billing logic regressed.
- **The billing portal is visible to non-admins** — role-gating regressed.
- **The Enterprise tier appears as a separate plan** — tier model regressed.

## Scenarios

- [`scenarios/billing-seats.feature`](scenarios/billing-seats.feature) — the
  tier model, active contributor metering, plan selection, billing portal
  (all `NO TEST` — nothing built).

## Open gaps

- **Active contributor definition** — how is "active" defined (seat metering)?
- **Fair-use limits, storage caps, non-profit/education pricing?**
- **Payment processor integration** — none selected.
- **Metering aggregation** — daemon-side not built.

## Related

- [features/07-pricing.md](../../../../features/07-pricing.md) — pricing
  feature doc (mock tier table, flows, status table).
- [design/dojo-web.md](../../../../design/dojo-web.md) — org context billing
  surface (admin).
- [mockups/Sensei/lib/dojo/dojo-billing.jsx](../../../../mockups/Sensei/lib/dojo/dojo-billing.jsx)
  — visual source (tier table + per-seat example, hardcoded data).
- [mockups/Sensei/lib/dojo/dojo-saas.jsx](../../../../mockups/Sensei/lib/dojo/dojo-saas.jsx)
  — SaaS entry: sign-in · orgs · create Dōjō (plan choice).
- [prd/01-vision.md §Business model](../../../01-vision.md) · [prd/03-personas.md §4,5](../../../03-personas.md).
- Crates: **none yet** — Worker: `dojo/src/routes/(app)/org/[slug]/billing/`.