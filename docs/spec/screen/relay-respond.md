# 答 · Relay · Respond

**Segment:** 05 · Relay (mobile companion)
**Route:** Relay iOS app · respond / decide gate (no web route — phone app)
**Source mockup:** [`lib/relay/relay.jsx`](../../mockups/Sensei/lib/relay/relay.jsx) → `RelayRespond`
**Data:** greenfield — a **decide gate** raised by the coordinator, delivered over the relay as a small chat. Proposed: `GET /api/relay/agents/{id}/thread` → `{ title, machine, assistant, turns: [{role:'agent'|'you', text}], pending?: { question, options: [{label, hint}] } }`; reply `POST /api/relay/gates/{id}` `{ choice?: <option index>, reply?: <free text> }`. Only the agent's *question*, the *option labels*, and the user's *reply* cross the relay — never code or transcripts.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + relay transport not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

The other shape of gate: not "approve this command" but "make a call." The
agent has reached a fork it can't resolve alone — a strategy choice, a
tradeoff — and it asks the way sensei asks: a short question, **3–4
quick-pick option chips** (each with a one-word hint like *safest* / *faster*),
and a **free reply** if none fit. This is a lightweight chat, keyboard up,
so the away-from-keyboard user can steer a long run with one tap or a
sentence. The user picks "additive · safest," adds a nuance in prose, and the
agent resumes — the whole round-trip in seconds, no code ever shown.

## Data invariants

- The thread carries **only** the agent's composed question, the option
  labels + hints, and the user's replies. **ZERO-KNOWLEDGE: no code, no
  file contents, no transcript excerpts** — the agent's message is a
  coordinator-composed decision prompt, not raw model output pasted in.
- The pending decision has **3–4 options** plus an always-available free
  reply (R4). Each option is `{label, hint}` where the hint is the terse
  tradeoff word (the mockup: `· safest`, `· faster`). Fewer than 2 options
  or more than 4 is malformed for a decide gate.
- Header names `title`, `machine`, and `assistant` (`macbook-pro · Claude`)
  so a multi-machine user knows which agent they're steering (R7).
- Turns alternate `agent` (left, paper bubble) and `you` (right, accent
  bubble). The agent's follow-up after a choice ("Got it — resuming with an
  additive migration.") is itself a status turn, content-free.
- Sending a **choice** posts the option index; sending a **free reply** posts
  the text. Either resolves the pending decision and the coordinator resumes
  the run. A free reply may accompany or override a chip (the mockup shows the
  user picking additive *and* adding "Keep the old columns for a week.").
- The composer is a single message field ("Message the agent…") with a send
  affordance; it is always available even when option chips are shown — the
  free reply is never gated behind the chips.

## Signals shown

Header:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Back | → [Task detail](relay-task-detail.md) / [Dashboard](relay-dashboard.md) | leave the thread | — |
| Title | `task.title` | which task's decision | `Refactor auth module` |
| Context | `machine · assistant` (mono) | which agent you're steering (R7) | `macbook-pro · Claude` |

Thread + decision:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Agent question | left bubble (paper) | the fork it can't resolve alone | `I need to pick a migration strategy. Which do you want?` |
| Option chip | `label` + muted `hint` | a quick-pick answer (3–4) (R4) | `Additive & reversible · safest` |
| Option chip | `label` + muted `hint` | alternative | `In-place rewrite · faster` |
| Your reply | right bubble (accent) | your choice / nuance | `Go additive. Keep the old columns for a week.` |
| Agent ack | left bubble (paper) | status turn, resuming | `Got it — resuming with an additive migration.` |
| Composer | free text field + send | reply when no chip fits | `Message the agent…` |

Real mockup content: question "I need to pick a migration strategy. Which do
you want?"; options `Additive & reversible · safest` and `In-place rewrite ·
faster`; user reply "Go additive. Keep the old columns for a week."; ack "Got
it — resuming with an additive migration."

## Done gate

The decide gate renders as a lightweight chat: the coordinator's composed
question, **3–4 option chips** each with a terse tradeoff hint, an
always-available free-reply composer, and the running thread of agent/you
turns — with the keyboard up for immediate steering. Picking a chip or
sending free text resolves the decision and resumes the run; nothing but the
question, labels, and replies crosses the relay.

## Wrong gate

- **The agent's message contains code, a diff, or a transcript excerpt.**
  A decide prompt is a composed question, not raw model output — zero-knowledge
  violation (R5).
- **Only a yes/no, or 5+ options, or no free-reply path.** A decide gate is
  **3–4 options plus a free reply** (R4); collapsing it to a binary or
  removing the escape hatch breaks the "the way sensei asks" contract.
- **The composer is disabled while chips are shown**, forcing the user into a
  canned answer when none fits.
- **Option hints are missing**, so the user can't see the tradeoff (safest vs
  faster) that makes a quick pick safe.
- **The reply doesn't resolve the gate / resume the run** — the round-trip
  dead-ends and the agent stays stalled.
- **No machine/assistant context**, so a multi-agent user can't tell which run
  they're answering (R7).

## Related

- Objectives [R4](../../objectives.md#relay--supervising-long-runs-from-anywhere) (a decision as **3–4 options + a free reply**), [R5](../../objectives.md#relay--supervising-long-runs-from-anywhere) (zero-knowledge)
- [architecture/relay](../../architecture/relay.md) — gate shape **decide** (3–4-option question plus a free reply — the way sensei asks)
- [journeys/relay](../../journeys/relay.md) — `gate: decide → 3–4 options + a free reply`
- Sibling relay screens: [Approve](relay-approve.md) (the *approve* gate) · [Task detail](relay-task-detail.md) · [Dashboard](relay-dashboard.md) · [Security](relay-security.md) · [Pairing](relay-pairing.md)
