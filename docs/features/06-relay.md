---
name: Relay
type: feature
kind: functional
---

# Relay

Relay is remote processing — how a user handles work away from the keyboard.
The user authors a phased plan on their machine; the daemon executes it while
they watch, approve the gates that matter, and steer from their phone. Code and
transcripts never leave the machine — only status, and the decisions the user is
asked to make.

> **Status of record → [`coverage-map.md`](coverage-map.md).** P0–P4 + seat
> attribution shipped on `develop` (drive/gate flags OFF, not merged to `main`).
> The status table below predates that; see the coverage map §C for the
> authoritative relay built-vs-pending, and `docs/design/agentic-execution-team.md`
> for how relay becomes the execution substrate for the planned agentic team.

## Flows

1. **Plan.** Write a deep, phased plan (or use the planner); mark which steps
   auto-run and which need a gate.
2. **Execute.** The daemon runs the plan feature by feature. It pauses on limits
   and resumes, recovers from crashes, and makes safe assumptions rather than
   stalling to ask.
3. **Watch.** From the phone or the console, see filtered status — feature
   shipped · gate needed · paused until <T>. No code, no diffs.
4. **Decide (HITL).** Approve or decline a gate; nudge to steer or unstick
   (retry · skip · resume · force-advance).

## Mockups

- [Phone — run dashboard · detail · gate · nudge](../mockups/Sensei/lib/relay/relay.jsx) · [phone planner](../mockups/Sensei/lib/relay/relay-planner.jsx)
- [Desktop — relay coordinator + plan author](../mockups/Sensei/lib/relay/relay-desktop.jsx)
- [Console — inbox · watch · PR-review · nudge](../mockups/Sensei/lib/dojo/dojo-relay.jsx)
- Design: [`plan/relay-engine.md`](../plan/relay-engine.md) · journey: [`journeys/relay.md`](../journeys/relay.md)

## What's involved

> What the user sees and does. `- [x]` done · `- [~]` partial · `- [ ]` not started.

### Remote planning

- [x] Author a phased plan (a plan doc, or the planner UI)
- [x] Mark each step auto or gate — which stops need a human
- [x] A depth check before an unattended run — catch under-specified steps upfront

### Autonomous execution

- [x] The daemon owns the run — it keeps going after the laptop session ends
- [x] Pauses on rate / weekly limits, then auto-resumes
- [x] Watchdog + crash recovery
- [x] Progress over asking — makes safe assumptions and logs them, rather than stall
- [~] Driving live sessions is behind a flag (off by default)

### Status check — from anywhere

- [x] Phone run dashboard — projects, phase progress, status pills (running · stalled · needs you)
- [x] Run detail — stage checklist + activity timeline
- [x] Filtered feed — no code, no diffs leave the machine
- [x] Offline-readable, with reconnect drafts + an action queue
- [~] Push when something's blocked on you (built; not enabled in production)

### Human-in-the-loop

- [x] Gate card — approve or decline a destructive action (with a dry-run option)
- [x] Nudge — steer or unstick (retry · skip · resume · force-advance)
- [x] PR-style review per segment — approve · request changes · comment
- [~] Gates on live sessions — the blocking hook is built but off / not registered by default

### Under the hood

- The control channel is the assistant's hooks: a blocking pre-tool-use hook holds at a gate, and the daemon injects nudges on the next hook fire. Full design in [`plan/relay-engine.md`](../plan/relay-engine.md).

## Status

> P0–P4 are shipped on `develop` (not yet merged to `main`); execution driving
> and gates ship behind flags that are **off by default**.

| Area | Status | Notes |
|---|---|---|
| Remote planning (plan + mark gates + depth check) | Done | planner + depth-reviewer |
| Autonomous execution (daemon run engine) | Done (flag off) | run tick, pause/resume, watchdog, progress-over-asking; driving off by default |
| Phone status (run list · detail · feed) | Done | filtered, offline-readable |
| HITL gate + nudge + PR-review | Done | phone + console |
| Away-from-keyboard push | Partial | web push shipped; VAPID / realtime not in production |
| Live-session gates (hook) | Partial | built, off / not registered by default |
| Multi-assistant (beyond Claude) | Not started | future |
| Team relay (shared inbox / presence) | Not started | folds into a later dōjō phase |
</content>
