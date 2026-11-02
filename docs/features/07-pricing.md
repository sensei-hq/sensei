---
name: Pricing
type: feature
kind: business
---

# Pricing

Pricing is meant to stay simple and fair. The user never pays to *use* sensei,
and never pays for tokens — inference is bring-your-own-key and runs locally.
What you pay for is the dōjō holding **private code's knowledge** — a private
repository's insights, metrics and diagrams in a dōjō.

## What the plan decides (D46)

The rule is [D46](../roadmap/09-dojo-expansion.md) (user, 2026-11-02):

- **Sharing is what puts a repository in the dōjō.** A shared repository's
  insights, metrics and diagrams can be shown there; an unshared one cannot.
- **Public repositories are in the free plan.** An organisation's public
  repositories sync automatically — members have no toggle; an admin may turn
  one off as org policy. A personal dōjō's public repositories are the person's
  to choose, all of them by default, and an opt-out holds.
- **Private repositories need a subscription** — an organisation's and a
  person's alike — taken by the dōjō's admin or owner. Until there is one they
  are **hidden** from the dōjō's repository lists, which say instead that private
  repositories are supported on paid plans and link to **Settings › Plan & billing**.
- **Every dōjō has Settings › Plan & billing**, organisation or personal. It
  reads **Free** until a subscription exists.
- **The plan lives only in the dōjō.** Local sensei — the desktop app, the
  daemon, the CLI and MCP — works in full on any plan. A subscription is needed
  only for sync and the dōjō's benefits (gating, integrations, consolidated
  metrics — D24).
- **The dōjō's per-repository election is the one authority** for what syncs.
  The local `sensei.repositories.visibility` gate is retired.

## Tiers

Three tiers, kept deliberately simple:

- **Free — every dōjō, public repositories.** Full local sensei, and a dōjō
  (organisation or personal) for your public repositories, free forever.
- **Paid — private repositories.** A subscription on the dōjō, taken by its
  admin or owner, brings its private repositories in, along with the dōjō's
  benefits: role consoles, shared governance and knowledge, client engagements,
  audit, and team relay. For an organisation it is priced per active
  contributor — read-only members are free.
- **Support — GitHub Sponsors.** Optional and separate from the product; sponsor
  development at [github.com/sponsors/sensei-hq](https://github.com/sponsors/sensei-hq).

> **Keep it simple.** The mock has a separate **Enterprise** tier (self-host,
> SSO / SCIM, air-gapped bundle, audit retention, SLA). We fold that into the
> paid org tier as options rather than carry a third plan.

## Flows

1. **Individual / OSS.** Install and use sensei free, on any plan. A personal
   dōjō shows your public repositories free; its private ones appear once you
   subscribe in its Plan & billing.
2. **Team.** An organisation's dōjō arrives on the free plan with its public
   repositories syncing. An admin subscribes in Plan & billing to bring the
   private ones in; billed per active contributor.
3. **Support.** Anyone can sponsor development through GitHub Sponsors.

## Mockups

- [Plan & billing](../mockups/Sensei/lib/dojo/dojo-billing.jsx) — the tier table + per-seat example
- [SaaS entry — sign-in · orgs · create dōjō (plan choice)](../mockups/Sensei/lib/dojo/dojo-saas.jsx)
- Website support block: `website/src/routes/sensei/+page.svelte` (GitHub Sponsors)

## What's involved

> `- [x]` done · `- [~]` partial · `- [ ]` not started.

- [ ] **Free — public repositories** — local sensei in full on any plan; every dōjō's public repositories free forever (D46)
- [ ] **Paid — private repositories** — a subscription by the dōjō's admin or owner, organisation or personal; per active contributor for an organisation (read-only free); the Enterprise extras (self-host · SSO/SCIM · audit retention · SLA) become options here, not a separate plan
- [ ] **Private repositories hidden until subscribed** — repository lists say private repositories are supported on paid plans and link to Plan & billing (D46)
- [ ] **Plan & billing in every dōjō** — organisation and personal; reads Free until a subscription exists (D46)
- [x] **GitHub Sponsors** — optional support; live link on the website today
- [ ] **Plan selection** in the dōjō's Plan & billing (a dōjō starts on the free plan — D20)
- [ ] **Billing & metering** — active-contributor detection, invoices, payment processor

## Status

| Area | Status | Notes |
|---|---|---|
| Free (public repositories) | Not started | intent; website says "free during preview" |
| Paid (private repositories) | Not started | mock tier table only; no billing integration |
| Enterprise → fold into paid | Decision | simplify: keep the extras as options, drop the third plan |
| GitHub Sponsors | Done | live link on the marketing website |
| Billing / metering / invoices | Not started | mock uses hardcoded example data |

## Open questions

- How is an "active contributor" defined (seat metering)?
- How is a personal dōjō's subscription priced (one person, no seats)?
- Self-host: license vs. subscription?
- Fair-use limits, storage caps, non-profit / education pricing?
</content>
