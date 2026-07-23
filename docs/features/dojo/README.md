---
name: Dōjō
type: feature
kind: functional
---

# Dōjō — what you see

> Your team's shared plane, layered over your personal sensei. This is what you
> expect to see as a user, and what each role adds — across the two planes:
> **governance** (the rules and guidance that shape the work) and **relay**
> (watching and steering runs away from the keyboard). The screen/route design
> lives in [`../../design/dojo-web.md`](../../design/dojo-web.md); the deeper plane
> docs are [Governance](../05-governance.md) and [Relay](../06-relay.md).

You always land in **your own work**. A dōjō is optional — you join or create one
when you want to share what you learn with a team.

## As a user — what everyone sees

You sign in and land on **your work**: the projects you're working on, anything that
needs you, and your live runs. A **"my dōjōs"** list shows the teams you belong to and
**your role** in each (empty until you join or create one). Everything below stays
within reach whatever org you're looking at.

**Governance plane — personal.** Even solo you get a constitution: the rules sensei
follows on your projects. You can

- adopt recommended rules — proven packs (principles · security · language/stack ·
  compliance · design) — into your own constitution;
- preview, for any project, exactly which rules govern it and why, before the first commit.

**Relay plane — personal.** For work running away from your keyboard you can

- watch a run's progress;
- get pinged when something **needs you**;
- approve a gated action, answer a decision, or chat to steer — only status and the
  choices you're asked to make cross the line, never code or transcripts.

## By role — what a dōjō adds

Inside a dōjō you belong to, what you can *do* depends on your role. Roles are
**additive** — you can hold more than one — and personal is always still there.

- **Developer** (default) — read-mostly. See the dōjō's projects and preview their
  constitutions; see your teams, what you've contributed upstream, and the approved
  knowledge that flows back down to you.
- **Maintainer** — owns the **governance plane** at org level: curate and author the
  dōjō's rules and stance along the ladder, triage incoming findings, approve what gets
  shared, and manage the published knowledge.
- **Lead** — guards client work: define client engagements, keep confidentiality (share
  the lesson, never the source), and hold the audit trail.
- **Admin** — runs the org: change member roles and access policies, connect
  identity/SSO, and manage the plan.

## The governance ladder — how rules resolve

Rules resolve broad → specific: **company → client → personal → project → stack**. The
more specific scope refines the broader one — except a rule marked **non-negotiable**,
which locks and can't be relaxed beneath it. Confidentiality always resolves first.

On a **client engagement** (you're on one company's payroll, working on another org's
repo) both constitutions apply: the **owning org wins ordinary conflicts**, while your
employer's non-negotiables remain a floor you can tighten but never relax.
