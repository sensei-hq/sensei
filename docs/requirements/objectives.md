# Objectives

> The **WHAT**, broken down. Each objective states the outcome and how we know
> it's met — never the implementation. Architecture
> ([`../architecture/`](../architecture/README.md)) maps each to a layer; the
> per-screen/per-pipeline contracts live in [`../spec/`](../spec/README.md).
> Traces back to [`vision.md`](vision.md).

Objectives are grouped by the four personal segments plus the cross-cutting
Dōjō layer. Every one is judged against the north-star: **does it move FTR, or
expose why it moved?**

---

## 01 · Bootstrap 支

**Reach a working, trustworthy environment without thinking about toolchains.**

| # | Objective | Met when |
|---|---|---|
| B1 | Verify what's already present before changing anything | Homebrew · Postgres · Ollama · daemon are probed and reported, no blind installs |
| B2 | Bring every prerequisite up **green** | A single status surface shows all deps healthy or the exact remediation |
| B3 | Never leave the user stuck | Every red state names the fix (and, where safe, offers to run it) |

## 02 · First run &amp; Preferences 名

**Point sensei at real folders, watch projects appear, then tune what defaults got wrong.**

| # | Objective | Met when |
|---|---|---|
| F1 | **Value before setup** (theme 1) | First interaction shows the user's own discovered projects — not a wizard |
| F2 | Discover real projects from real folders | Pointing at `~/Developer` yields correctly-classified repos (one repo = one project = one owner) |
| F3 | Tuning is available, never blocking | Every default (scan roots, assistants, providers, inference, libraries) is adjustable in a searchable Preferences surface |

## 03 · Observatory — daily use 家

**Walk in, learn the one thing that needs me today, act on it, stay in control of what leaves my machine.**

| # | Objective | Met when |
|---|---|---|
| O1 | Surface *today's one thing* | The landing shows a single highest-value action with its receipt, not a wall of metrics |
| O2 | One decision, one default (theme 2) | Insights/traceability/libraries/upgrades all use **Apply · Review · Dismiss** |
| O3 | Every claim carries a receipt (theme 4) | FTR chips, confidence, before/after — the user can verify, not trust |
| O4 | The user controls what leaves the machine (theme 5) | Nothing is shared without an explicit Dōjō membership + preview |
| O5 | The module loops each offer an action | Security · Architecture · Testing · Style · Memory · Traceability · Impact · Libraries · Insights each observe → find → offer one action |

## 04 · The project window 雲

**Work inside one project end-to-end and trust what sensei learned here before any of it travels.**

| # | Objective | Met when |
|---|---|---|
| P1 | A trustworthy per-project overview | Overview shows FTR, top recommendation, memory/drift counts — all real, no fabricated numbers |
| P2 | Learned knowledge is inspectable | Memories, patterns, traceability, libraries, impact are all viewable with provenance |
| P3 | Nothing travels unseen | Sharing a finding upstream always previews what leaves and what's stripped |

---

## Dōjō — the cross-cutting team layer

**Extend the same retrospective loop across a team without leaking client work.**
The Dōjō is not a linear "segment 5" — it threads through the Observatory and the
project window (in-app developer surface) and adds an external SaaS **console**
for maintainers/admins/leads. Its core principles (from the journey map):
**identity is the discovery · pull never push · never blind, always preview ·
one universal strip · specificity wins conflicts.**

**Membership contexts** — one developer, many orgs: *Employer · Client orgs ·
Communities · Personal.* The org boundary is the **membership**, with
client-precedence as the tiebreaker.

### Developer (in-app)

```mermaid
flowchart LR
    d1[Discover<br/>my org runs a Dōjō] --> d2[Authenticate<br/>SSO · OAuth · device-code]
    d2 --> d3[Bind project → org] --> d4[A finding forms<br/>memory / pattern / guard]
    d4 --> d5[Share upstream<br/>right attribution + confidentiality]
    d5 --> d6[Watch it travel] --> d7[Receive downstream<br/>approved org knowledge where I work]
```

### Maintainer (console)

```mermaid
flowchart LR
    m1[Open the queue<br/>grouped by scope, ranked] --> m2[Evaluate a candidate]
    m2 --> m3[Decide<br/>approve · revise · decline, with a trail]
    m3 --> m4[Set distribution<br/>who receives it] --> m5[Publish &amp; measure<br/>did it land + help]
```

### Org admin (console)

```mermaid
flowchart LR
    a1[Stand up<br/>our own infra] --> a2[Connect identity<br/>OIDC / SAML + git OAuth]
    a2 --> a3[Provision members<br/>right roles, fast] --> a4[Scopes &amp; policies<br/>hierarchy + attribution/confidentiality]
    a4 --> a5[Monitor<br/>keep the Dōjō healthy]
```

### Lead — client / engagement confidentiality (console)

> The role formerly called *client-lead* is now **lead**. It guards
> confidentiality on **client engagements** (the engagement concept is
> unchanged; only the role name shortened).

```mermaid
flowchart LR
    l1[Define engagement<br/>register a client, route correctly] --> l2[Anonymize, always<br/>share the lesson, never the source]
    l2 --> l3[No per-item review<br/>trust the universal strip] --> l4[Audit trail<br/>prove confidentiality held]
    l4 --> l5[Incident handling<br/>contain a near-leak fast]
```

### Dōjō lifecycle — what flows through it

```mermaid
flowchart LR
    c1[contribute] --> c2[accumulate] --> c3[triage] --> c4[approve] --> c5[distribute]
    c5 -.->|received downstream| c1
```

| # | Objective | Met when |
|---|---|---|
| DJ1 | Personal sensei works with **no** Dōjō | Every local capability functions with zero memberships |
| DJ2 | The org boundary is exact (theme 5) | A lesson from a client engagement never leaves without anonymization + preview |
| DJ3 | Pull, never push | Downstream knowledge arrives by opt-in pull, never forced |
| DJ4 | Attribution + confidentiality are automatic | One universal strip runs on every client lesson; a lesson that can't be stripped doesn't leave |
| DJ5 | Distribution respects priority ladders, not a rigid hierarchy | Specificity wins conflicts; memberships route by precedence |

---

## Cross-cutting (apply to every objective)

- **Insight copy from the model** (theme 6) — user-facing strings route through `insight-copy`.
- **Discoverability of depth** (theme 3) — nothing important hidden behind a one-liner.
- **The pair goes both ways** — objectives capture human-side signal (sparse instructions, wrong assumptions), not only assistant errors.

## Read next

- [`open-issues.md`](open-issues.md) — how far the implementation is from these objectives, ranked, with the plan.
- [`../architecture/README.md`](../architecture/README.md) — the layers that deliver them.
