---
name: Governance
type: feature
kind: functional
---

# Governance

> The implementation / design name for this feature is **dōjō** — a shared
> control plane for a team or organization.

A solo user's sensei is local and private. Governance is what lets a team or
organization share standards, knowledge, and controls across everyone's work —
and keep private knowledge coordinated. It gives a group three things:

1. **A shared observatory control plane** — the team's collective view: what's
   flowing in, what's been adopted, what needs a decision.
2. **Shared knowledge** — stack / technology-specific patterns, guards, skills,
   and agents, curated once and distributed to everyone.
3. **Client / org / team guidelines and controls** — governance rules, scopes,
   identity, and audit, tuned per team or client.

The line is drawn by who the knowledge is *for*, not by features: a dōjō
coordinates a group's private knowledge (see [Pricing](07-pricing.md)).

Two shared planes sit above the individual: the private **dōjō** (attributed,
governed, per org / team / client) and the public **Collective** (an anonymized
community commons). A finding is shared into one or the other — always opt-in,
always previewed, and client work is anonymized before it can leave.

The team journey is one loop: **bind** a project → **contribute** a finding →
maintainers **triage & approve** (high-impact needs a second approval) →
approved knowledge **distributes** back, routed by scope (more specific wins).

## Flows

1. **Bind & join.** Connect a project to a dōjō; membership comes from the git org.
2. **Contribute.** A finding forms in a project and is shared upstream — previewed, opt-in.
3. **Triage & approve.** Maintainers weigh candidates (evidence · conflicts · duplicates), decide, and set how far it distributes.
4. **Distribute.** Approved knowledge flows to members, scoped; precedence resolves conflicts.

## Mockups

- [Dōjō journey](../journeys/dojo.md) — bind · contribute · triage · distribute (business model + open questions)
- [Developer console](../mockups/Sensei/lib/dojo/dojo-developer.jsx) · [Maintainer console](../mockups/Sensei/lib/dojo/dojo-maintainer.jsx) · [Lead console](../mockups/Sensei/lib/dojo/dojo-lead.jsx) · [Admin console](../mockups/Sensei/lib/dojo/dojo-admin.jsx)
- [Governance authoring](../mockups/Sensei/lib/dojo/dojo-governance.jsx) · [Identity & SSO](../mockups/Sensei/lib/dojo/dojo-identity.jsx)
- [In-app touchpoints (join · bind · share · downstream)](../mockups/Sensei/lib/dojo/dojo-inapp.jsx) · [SaaS entry (sign-in · orgs)](../mockups/Sensei/lib/dojo/dojo-saas.jsx)

## What's involved

> What each role sees and does. `- [x]` done · `- [~]` partial · `- [ ]` not started.

### Roles & consoles

- [~] **Developer** — My teams · My contributions · For me: your memberships, the status of what you've contributed, and the approved org knowledge coming to you (scope-badged, precedence resolved).
- [~] **Maintainer** — Triage · Approvals · Knowledge · Catalog: work the queue, weigh candidates (evidence · conflicts · duplicates), approve / revise / decline, set distribution, then publish and measure.
- [~] **Lead** — Clients · Audit: define a client engagement (anonymization locked first), watch the immutable audit trail, handle incidents.
- [~] **Admin** — Overview · Monitor · Members · Scopes · Governance · Identity · Plan & billing: stand up the dōjō, connect SSO, provision members, define scopes and rules, and monitor adoption.

### 1 · Shared observatory control plane

- [~] Monitor — throughput, adoption, leak-guard, recent activity, the published library
- [~] Collective decision queue — triage + approvals across the team
- [~] Downstream distribution — approved knowledge back to members

### 2 · Shared knowledge — stack / technology-specific patterns

Six kinds of thing travel through a dōjō: **guiding principles**, **patterns**
(and anti-patterns), **prompts**, **guards**, **skills**, and **agents** (plus
commands).

- [~] Curate and publish those artifacts at a scope
- [~] Collective ↔ dōjō toggle in-app, with precedence when they conflict
- [ ] Scope-scoped authoring of rules / skills / agents / commands + memory (mock only)
- [~] Personal dōjō — the same authoring, sized for one, across your linked emails
- [~] Inherit-on-bind — a new member's project picks up the composed bundle (rules · skills · agents · commands · memory) on day one

### 3 · Client / org / team guidelines & controls

- [x] Scopes & precedence ladder — org → team → project → repo · stack; more specific wins
- [~] Client engagements — universal anonymization, no per-item review
- [ ] Governance stance per scope — autonomy · sharing · review · anonymization; preview what a new member inherits (mock only)

### Identity & membership

- [~] Sign in — GitHub (primary), magic-link (non-GitHub orgs), or device-code (CLI / headless agents); no anonymous access
- [~] One account, many linked emails; memberships never cross (one org can't see your others)
- [~] Namespaces — `github/<org>` · `other/<org>` · `personal/<you>`
- [~] Roles derived from git access (highest across repos), capped at read-only, hand-elevated to maintainer / admin; SCIM + JIT provisioning
- [~] Membership types — employer (社) · client (客) · community (群) · personal (己); a project binds to exactly one, findings route only there, and client takes precedence

### Confidentiality, attribution & audit

- [~] By origin — open-source (public credit, nothing stripped) · personal-closed (named to you, specifics stripped) · employer (org-internal) · client (source-dereferenced, anonymized)
- [~] Client work auto-anonymized — drop client · repo · identifiers · source; keep the lesson (what · why · impact); if it can't be anonymized, it doesn't leave
- [~] Never blind — a raw-vs-stripped preview before every share; recallable in triage, retractable after adoption
- [~] Leak-guard — quarantine anomalous outbound; immutable, hash-chained audit; incidents; retention per engagement

### The Collective — the public commons

- [~] A public, anonymized knowledge commons (community cloud), separate from the private dōjō
- [~] Opt-in share up, pull vetted upgrades down; reputation + aggregate signal, not named approval
- [~] Per-category filters and cadence; promote knowledge from your company out to the world

### Rules, promotion & metrics

- [x] Governance rules — three tiers: **mandatory** (pinned, non-overridable), **scoped** (guards · patterns · principles), **promoted** (proposed from playbook-learning outcomes); resolved live
- [~] Promotion — playbook → outcome proposals; the front door tunes itself, governed like any rule
- [ ] Delivery metrics (DORA) → feed the planner

## Status

| Area | Status | Notes |
|---|---|---|
| Role consoles (developer · maintainer · lead · admin) | Partial | dōjō web app console screens shipped; several /v1 data paths still being wired |
| Bind & connect (in-app) | Partial | connect + sharing shipped; membership validation has gaps |
| Governance rules (mandatory · scoped · promoted) | Done | resolved live; promotion via proposals partial |
| Shared knowledge / collective intelligence | Partial | distribution partial; curation + authoring mock |
| Governance authoring (stance · rules · skills · memory) | Not started | mock-only |
| Scopes & policies | Partial | precedence ladder designed; wiring in progress |
| Identity / SSO / SCIM / members | Partial | screens shipped; SSO/SCIM wiring in progress |
| Memberships & routing | Partial | employer / client / community / personal; one binding; client precedence |
| Confidentiality & anonymization | Partial | origin-based stripping, leak-guard, never-blind preview; wiring in progress |
| The Collective (public commons) | Partial | anonymized commons; largely design + the in-app toggle |
| Personal dōjō | Partial | governance-for-one across linked emails |
| Client engagements + audit | Partial | Lead console + immutable audit designed; wiring in progress |
| Delivery metrics (DORA) | Not started | feeds the planner (future) |

> The dōjō web app runs as a Cloudflare Worker at `dojo.sensei-hq.com` (console
> chrome + org picker are real; many console data paths are still being ported).
</content>
