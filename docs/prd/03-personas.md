# Personas

Who sensei serves, their goals, and which modules serve them. Every module
spec's **Serves:** line points back here (reorg §3.3). Authored 2026-10-13
per decision 10 — including the **management** and **client / workspace**
viewpoints that the multi-user, multi-team case needs.

Two named residents came from the mockups' own copy: **Rin** — a developer's
day through the end-to-end journey — and **Keiko** — an org's practice. They are
used below where the journey docs named them.

> **Naming note (decision 10).** The grouping vocabulary — pods, swimlanes,
> squads, teams — is deliberately left open. A person holds many *scopes*
> (memberships in many Dōjōs and workspaces); what to call a group is
> a display concern, not a model one.
>
> **Not a persona:** an AI assistant is a *delivery channel*, not a person whose
> goals we design for. Its constraints (context budget, tool surface, adapter
> behaviour) are acceptance criteria in M3/M14, never personas here.

---

## Core set

### 1 · Solo developer — *"the wedge"*

> The free, personal, local loop. Everything else is built on this being great.

| | |
|---|---|
| Context | One person, one machine, several real repos. No team, no org, no Dōjō. |
| Goals | Stop losing what he learned with his AI assistant between sessions and between workspaces. |
| Jobs-to-be-done | Install once and forget (M1); point sensei at real folders and see workspaces appear (M2); get *today's one thing* on opening the app (M15); trust what sensei learned about one workspace before anything travels (M16); see and replay what the assistant actually did (M18). |
| "Good" feels like | He opens the Observatory, sees one high-value action with its receipt, acts, and the loop visibly closes (M19). No wall of metrics, no wizard, no fabricated numbers. |
| Served by | M1 · M2 · M3 · M4 · M7 · M8 · M14 · M15 · M16 · M17 · M18 · M19 |

### 2 · Team developer — *"the inheritor"*

> In a team that runs a Dōjō. Inherits org knowledge; contributes upstream.

| | |
|---|---|
| Context | Works in a Dōjō membership (employer, community, or client org). Her preferences, sessions and workspace binding all ride her identity. |
| Goals | Start fast on a repo the team has history on; contribute without leaking client work; get governed knowledge where she works. |
| Jobs-to-be-done | Discover her org's Dōjō and authenticate (SSO / OAuth / device-code) (T1/T2); bind workspace → org (T1); share findings upstream with right attribution and confidentiality (T1); receive approved downstream knowledge (T2). |
| "Good" feels like | Conventions are taught once and she inherits them on join; specificity wins over a rigid hierarchy; nothing she shares leaves without a preview and strip. |
| Served by | M3 · M9 · M13 · M14 · M15 · M16 · T1 · T2 |

### 3 · Maintainer — *"the curator"*

> Gates what becomes shared knowledge.

| | |
|---|---|
| Context | A maintainer console over a scope: queue → evaluate → decide → set distribution → publish and measure. |
| Goals | Keep the shared mind clean and current without it becoming a chore. |
| Jobs-to-be-done | Open the queue grouped by scope and ranked (T2); evaluate a candidate with its evidence (T2); approve · revise · decline with a trail (T2); set who receives it and measure whether it landed (T2, M19). |
| "Good" feels like | Triage is a judgement call, never a slog; every decision is recorded; distribution respects the priority ladder. The curation load stays proportional to the team (reorg: "triage can't become a chore"). |
| Served by | M10 · M13 · M19 · T1 · T2 |

### 4 · Org admin — *"the plane owner"*

> Identity, members, policies, audit, health.

| | |
|---|---|
| Context | Runs the org's private Dōjō (hosted). |
| Goals | Stand up the Dōjō, connect identity, provision members, keep the org data inside the boundary, monitor health. |
| Jobs-to-be-done | Subscribe (T4); connect identity — OIDC / SAML + git OAuth (T1/T2); provision members with the right roles fast (T2); set scopes and policies (T1, M13); monitor the Dōjō (T2). |
| "Good" feels like | Provisioning is fast, the org boundary is exact, and the audit trail proves confidentiality held. Payments appear only here, for the benefits of the paid tier — gating, integrations, consolidated metrics (T4, D24). |
| Served by | M13 · T1 · T2 · T4 · T6 (grants) |

### 5 · Engagement / client lead — retired (D34)

> **Retired 2026-10-22** (#307 R7, D34). A client is just another dōjō — an
> agency's people are members of the client's dōjō, and its work is that dōjō's
> Epics, Sprints and Tasks from its integrations. There is no engagement model,
> no incident register and no confidentiality ledger for a lead to run. What this
> persona relied on stays for everyone: the lesson travels, the source never
> does — the universal strip on every publish (T1), not a role's job.

---

## Extended viewpoints — the multi-user, multi-team case

### 6 · Manager / delivery lead — *"the roll-up"*

> Needs the birds-eye view without reading code; the customer for T3's sprint
> gates and the graph's L0.

| | |
|---|---|
| Context | Runs delivery across pods / swimlanes / workspaces / teams (whatever the org calls them — decision 10). Does not read code. |
| Goals | See status, risk and throughput across the org at a glance; know which workspaces are healthy and which are silently accumulating debt; hold quality gates without being unfair to inherited codebases. |
| Jobs-to-be-done | Read one roll-up of workspace health without per-repo archaeology (M8 master view); see module boundaries, coupling and hotspots — not symbol dust (M17 L0 birds-eye); see which sprint gates are blocking and why (T3); act only where the roll-up demands it. |
| "Good" feels like | Five minutes in the morning: green is green, red carries its reason, and a workspace that inherited a mess is measured against *its own baseline*, not an absolute target (reorg §9.4 ratchets). |
| What must be fair (decision 8) | Gates are delta-scoped (what this sprint introduced or chose to guard); absolute workspace-level ratios are improvement-only; thin-coverage metrics look thin, not strong (reorg §9.6). |
| Served by | M8 · M15 · M17 (L0) · M20 · T2 · T3 · T6 |

### 7 · Client / workspace stakeholder — *"the narrow lens"*

> Sees the same data as the lead through a workspace-scoped, confidentiality-held
> lens.

| | |
|---|---|
| Context | The client (or the workspace's sponsor) receiving a status on work done for them. May not be technical. Multi-team, multi-workspace orgs are where this shows up. |
| Goals | Trust that the workspace is progressing and healthy without seeing other clients' work or raw internals. |
| Jobs-to-be-done | See workspace-level progress and quality posture for *this* workspace (T3, T1 scope); understand what the team is claiming without reading code (M15/M16 through the lead's lens). Sees the workspace through a viewer grant (T6); cannot export. |
| "Good" feels like | The report she sees is the lead's, narrower by scope, and never contradicts the lead. Her confidentiality is invisible — it just holds. |
| Served by | M8 · M16 · T1 · T2 · T3 · T6 |

---

## How every module names its customer

A module spec's `## Serves:` line lists persona ids from this file — e.g.
`Serves: 1, 2 (M15 Observatory)`. A module with no persona on the list is a
suspicious module and must say why in its spec.

## Preference surface (decision 10)

Preferences are a persona concern, not a Module-2 footnote: the current app
serves everyone in **kanji** icons, while the mockups already support switching
between **kanji ⇄ solar** styles via rokkit's data-driven components. That toggle
was gap A-1 (reorg §12.10) and is resolved: the app stores it as `ui.glyphs`
(Settings → General), and the dōjō's account menu has Theme and Glyphs. Personal preferences must
extend to cover the configurations the app already supports (A-2). Personas
declare intent; the preference surface carries it.