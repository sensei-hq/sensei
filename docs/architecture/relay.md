# Layer · relay (coordinator · zero-knowledge relay · mobile)

> **Serves:** the Relay objectives R1–R8 — supervising long, multi-agent runs
> from anywhere, without leaking code. A cross-cutting capability spanning three
> planes; visual source [`../mockups/Sensei/Sensei Relay.html`](../mockups/Sensei/Sensei%20Relay.html),
> journey [`../journeys/relay.md`](../journeys/relay.md).

## What it is

Three planes and a human-in-the-loop round-trip. Agents run on **your own
hardware**; only a *filtered status* ever leaves it.

```mermaid
flowchart LR
    subgraph machine[Your machine(s)]
        AGENTS[agent CLIs<br/>Claude Code · Codex · OpenCode · Aider] --> COORD[coordinator<br/>supervises · runs the plan<br/>auto mode · raises gates]
    end
    COORD -->|filtered status only<br/>done · doing · next · gate| RELAY[[zero-knowledge relay<br/>encrypted · outbound-only]]
    RELAY --> PHONE[mobile companion<br/>dashboard · task · approve · respond]
    PHONE -->|approve / decide / nudge| RELAY --> COORD
    RELAY -.->|team: route to on-call| DOJO[Dōjō shared queue]
```

## The three planes

| Plane | Where | Owns |
|---|---|---|
| **Coordinator** | in/beside the [daemon](daemon.md); a new Observatory rail item | supervises the agent CLIs, runs the active plan in **auto (non-blocking) mode**, publishes the filtered status stream, raises **gates** when a step needs a human |
| **Zero-knowledge relay** | transport (relayed **through the Dōjō** for teams; the daemon stays **outbound-only**) | carries only *filtered status* + gate prompts + the human's replies — **never code or transcripts**; encrypted, paired, permissioned |
| **Mobile companion** | phone app | the away-from-keyboard surface: dashboard, task detail, approve, respond, security, pairing, + the planner |

## The planner model

Each **project carries one active plan**. A plan is modular:

```
project → plan → phase (n of x) → { feature · checkpoint · gate }
```

- **Phases** sequence the work; each holds **features** (the units of build),
  **checkpoints** (progress markers), and **gates** (steps that must stop for a
  human). Plan authoring (desktop) marks *which* steps gate.
- Development runs **non-blocking in auto mode**; the human sees project-level
  *done · doing · next* and is pulled in only at a **gate** or a **stall** (a
  **nudge**).
- **Gates** come in two shapes: **approve** (surfaces the *exact command* first)
  and **decide** (a **3–4-option question plus a free reply** — the way sensei
  asks).

## Mobile surfaces (from the mockup)

Dashboard (3-state, tasks grouped by machine) · Task detail (plan checklist +
activity timeline + gate) · Approve (exact command first) · Respond
(quick-pick / yes-no / reply) · Security (pairing + permissions) · Pairing
(onboarding round-trip) · Projects (one plan each, phases n of x + progress) ·
Plan (phases → features · checkpoints · gates) · Decisions (3–4 options + type
your own) · Nudge (a stalled track: done · doing · next).

## Security model

- **Zero-knowledge relay** — the coordinator publishes only filtered status; the
  relay is a dumb encrypted pipe. Agents + code never leave your hardware. Aligns
  with local-first and theme 5 (the org boundary is exact).
- **Encrypted pairing** — an onboarding round-trip establishes keys between the
  coordinator and the phone.
- **Scoped, revocable permissions** — the pairing declares what the relay may do
  (which projects, whether it can approve commands vs only observe). Prefer a
  vetted crypto/auth library over hand-rolled (kavach / the Dōjō auth stack).

## Team relay (Dōjō)

When agents run for an org, human-in-the-loop moments **fan into one shared
on-call queue**; a gate routes to whoever's on call, every decision carries
**attribution**, and it lands in the [Dōjō](dojo.md) record. Reuses the
sessions/projects/machines vocabulary.

## Status &amp; relation to the rest

Promoted from the deferred "ACP integration + control-plane / relay" decision
(see [`../plan/decisions.md`](../plan/decisions.md)). Net-new build: the
coordinator (supervise + publish + gate), the relay transport, the planner data
model, and the mobile app. The [daemon](daemon.md) grows the coordinator role;
the [app](app.md) grows the plan-authoring surface; [dojo](dojo.md) grows the
shared gate queue. Sequenced in [the plan](../plan/README.md).
