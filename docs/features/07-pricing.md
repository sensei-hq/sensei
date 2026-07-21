---
name: Pricing
type: feature
kind: business
---

# Pricing

Pricing is meant to stay simple and fair. The user never pays to *use* sensei,
and never pays for tokens — inference is bring-your-own-key and runs locally.
What you pay for is coordinating a **group's private knowledge** — a shared dōjō.
The line is drawn by who the knowledge is for, not by which features you get.

Three tiers, kept deliberately simple:

- **Free — individuals & open source.** Full local sensei and a personal dōjō
  for your own projects. Public / open-source repos are free forever.
- **Paid — organizations & teams.** A shared, private dōjō: role consoles,
  shared governance and knowledge, client engagements, audit, and team relay.
  Priced per active contributor — read-only members are free.
- **Support — GitHub Sponsors.** Optional and separate from the product; sponsor
  development at [github.com/sponsors/sensei-hq](https://github.com/sponsors/sensei-hq).

> **Keep it simple.** The mock has a separate **Enterprise** tier (self-host,
> SSO / SCIM, air-gapped bundle, audit retention, SLA). We fold that into the
> paid org tier as options rather than carry a third plan.

## Flows

1. **Individual / OSS.** Install and use sensei free; a personal dōjō needs no plan.
2. **Team.** Create a shared dōjō and pick the paid plan; billed per active contributor.
3. **Support.** Anyone can sponsor development through GitHub Sponsors.

## Mockups

- [Plan & billing](../mockups/Sensei/lib/dojo/dojo-billing.jsx) — the tier table + per-seat example
- [SaaS entry — sign-in · orgs · create dōjō (plan choice)](../mockups/Sensei/lib/dojo/dojo-saas.jsx)
- Website support block: `website/src/routes/sensei/+page.svelte` (GitHub Sponsors)

## What's involved

> `- [x]` done · `- [~]` partial · `- [ ]` not started.

- [ ] **Free — individuals & open source** — local sensei + a personal dōjō; public / OSS free forever
- [ ] **Paid — organizations** — shared private dōjō, per active contributor (read-only free); the Enterprise extras (self-host · SSO/SCIM · audit retention · SLA) become options here, not a separate plan
- [x] **GitHub Sponsors** — optional support; live link on the website today
- [ ] **Plan selection** at dōjō creation
- [ ] **Billing & metering** — active-contributor detection, invoices, payment processor

## Status

| Area | Status | Notes |
|---|---|---|
| Free (individual / OSS) | Not started | intent; website says "free during preview" |
| Paid (organization / team) | Not started | mock tier table only; no billing integration |
| Enterprise → fold into paid | Decision | simplify: keep the extras as options, drop the third plan |
| GitHub Sponsors | Done | live link on the marketing website |
| Billing / metering / invoices | Not started | mock uses hardcoded example data |

## Open questions

- How is an "active contributor" defined (seat metering)?
- Self-host: license vs. subscription?
- Fair-use limits, storage caps, non-profit / education pricing?
</content>
