# 信 · Relay · Coordinator

**Segment:** 05 · Relay (Observatory desktop)
**Route:** `/relay` (new Observatory rail item)
**Source mockup:** [`lib/relay/relay-desktop.jsx`](../../mockups/Sensei/lib/relay/relay-desktop.jsx) → `RelayCoordinator`
**Data:** _greenfield — the coordinator supervises local agent CLIs, tracks paired devices, publishes the filtered event stream, and holds pending gates; shapes proposed below_
**App file:** _greenfield — not built_ (the Observatory rail item `app/src/routes/(observatory)/relay/+page.svelte`)
**Daemon files:** _greenfield — coordinator not built_ (grows `crates/senseid`; new `coordinator` module + relay transport)
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md), objectives [R5 · R7](../../objectives.md#relay--supervising-long-runs-from-anywhere)

## Purpose

The coordinator is the **desktop hub** of the relay: the daemon
role that watches every agent CLI running on *this* machine, filters
their raw activity into safe events, and publishes only those — plus
the moments a human is needed — to the user's paired phones. This
screen is where the user, still at their desk, sees exactly what is
leaving the machine and can answer a pending gate without reaching
for the phone.

It is a **new Observatory rail item** (`信 · Relay`), so it lives
inside the Observatory window chrome and rail, reusing the
sessions/projects/machines vocabulary. The rail item carries a live
`accent` dot and a pending-gate count (`1` in the mockup) so the
user knows from anywhere in the Observatory that a gate is waiting.

Three panes answer three questions:

- **Paired devices** (rail footer) — what machines and phones are in this relay.
- **Published stream** — what filtered events have gone out, newest first.
- **Pending gate** — the one step waiting on a human, answerable here or on the phone.

Kanji is 信 — *signal / trust* (the relay carries a trusted signal, not the code).

This screen embodies **R5 (zero-knowledge by construction)** — it
must make visible *what is redacted before publish* — and **R7
(multi-agent, multi-machine)** — events and devices are grouped by
machine across Claude Code · Codex · OpenCode · Aider.

## Data invariants

The coordinator supervises agent CLIs on **this machine only**; it
never reaches onto another machine. Each paired machine runs its own
coordinator and publishes its own stream into the shared relay.

- **The coordinator is the only plane that touches code.** It reads
  raw agent activity locally and emits a *filtered event* — a
  redacted status record. Raw file contents, secrets, full diffs,
  and shell output **never enter the published stream** (the mockup
  states this literally as the redaction footer).
- **Published events are the wire format**, not the raw activity. A
  published event carries: `ts`, `machine`, `kind` (e.g.
  `auth · edit`, `tests · run`, `backfill · progress`), a short
  human-safe `detail` (e.g. `auth.ts +42 −8`, `Batch 42 / 120`),
  and a `severity`/tone (`gate | ok | live | edit`). The detail is a
  *summary line*, never a diff body.
- **Devices are paired**, not discovered. The rail footer lists
  paired devices with a health dot; a device appears only after the
  encrypted pairing round-trip (R6). Machines (macbook-pro,
  mac-mini) and phones (iPhone 16 Pro) are both devices; phones are
  the surfaces a gate can be relayed to.
- **A gate is a human-stop step** raised by the coordinator when a
  plan step marked `gate` (see [[screen/relay-plan-author]]) is
  reached, or when an agent hits an approval / question / account
  limit. Exactly the gate's *decision + context* is published —
  never the surrounding transcript. A gate carries the source
  (`machine · agent`), the exact command (for an **approve** gate)
  or the options (for a **decide** gate), and which phone(s) it was
  relayed to and when.
- **At most the pending gates are actionable here.** Answering a
  gate here or on the phone resolves the *same* gate — the decision
  is idempotent; whichever surface answers first wins, the other
  shows resolved.
- **The relay is outbound-only and blind.** The coordinator pushes;
  it is never inbound-reachable. The relay routes encrypted blobs by
  device key and holds no key that can open one.

Proposed shapes (greenfield — name the contract, don't fabricate values):

```
GET /api/relay/status →
{
  "relay": { "state": "live" | "paused" },
  "devices": [
    { "name": "macbook-pro",  "kind": "machine", "paired": true, "healthy": true },
    { "name": "mac-mini",     "kind": "machine", "paired": true, "healthy": true },
    { "name": "iPhone 16 Pro","kind": "phone",   "paired": true, "healthy": true }
  ],
  "agents_running": 3,
  "pending_gates": 1
}

GET /api/relay/stream?since=… →
{ "events": [
  { "ts": "14:10", "machine": "macbook-pro", "kind": "auth · gate opened",
    "detail": "Approve DB migration on production", "tone": "gate", "gate_id": "…" },
  { "ts": "14:08", "machine": "macbook-pro", "kind": "auth · test",
    "detail": "Ran suite — 18 passed", "tone": "ok" },
  { "ts": "14:05", "machine": "macbook-pro", "kind": "auth · edit",
    "detail": "auth.ts +42 −8", "tone": "edit" },
  { "ts": "14:03", "machine": "mac-mini", "kind": "tests · run",
    "detail": "Writing test_checkout.py", "tone": "live" },
  { "ts": "14:01", "machine": "mac-mini", "kind": "backfill · progress",
    "detail": "Batch 42 / 120", "tone": "live" }
] }

GET /api/relay/gates/pending → [
  { "gate_id": "…", "shape": "approve", "source_machine": "macbook-pro",
    "source_agent": "auth", "title": "Approve database migration",
    "command": "psql < 003_add_oauth.sql",
    "relayed_to": ["iPhone 16 Pro"], "relayed_ago_s": 40 }
]

POST /api/relay/gates/{gate_id}/decide  { "decision": "approve" | "deny", "reply"?: string }
POST /api/relay/publishing            { "paused": true | false }   # "Pause publishing"
```

## Signals shown

Header — eyebrow `信 · relay coordinator`, title **"What your agents
are relaying"**, sub-copy describing the filter-and-publish contract.
Right side: a live badge and a pause control.

| Element | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Relay-live badge | `zs-badge-success` + pulsing dot | Coordinator is publishing | `● relay live` |
| Pause publishing | secondary sm button | Stop the outbound stream (POST publishing) | `Pause publishing` |
| Stat · pending gate | count, `accent` | Gates awaiting a human | `1 pending gate` |
| Stat · agents running | count, `success` | Agent CLIs supervised now | `3 agents running` |
| Stat · machines | count, `ink` | Paired machines in the relay | `2 machines` |
| Stat · phones paired | count, `ink` | Phones that can receive gates | `2 phones paired` |
| Rail item badge | dot + count, `accent` | Pending gates, visible from any Observatory screen | `Relay ● 1` |
| Rail footer · Paired | device name + health dot | Paired devices (machines + phones) | `● macbook-pro · ● mac-mini · ● iPhone 16 Pro` |

**Published stream** (eyebrow `Published stream`) — one row per
filtered event, newest first:

| Element | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Timestamp | mono `HH:MM` | When published | `14:10` |
| Tone dot | color by tone, pulses when `live` | Event class | gate=accent · ok=success · live=ink-3 (pulsing) · edit=ink-2 |
| Source line | mono `machine · kind` | Which machine + agent event | `macbook-pro · auth · test` |
| Detail line | safe summary | The human-readable, redacted status | `Ran suite — 18 passed` · `Batch 42 / 120` |
| Gate row tint | `accent-soft` background | A gate event stands out in the stream | the `14:10` row |
| `needs you` chip | `zs-badge-accent` | This event is a gate awaiting a decision | on the gate row |
| Redaction footer | faint mono caption | Names exactly what never publishes | `Redacted before publish: file contents · secrets · full diffs · shell output` |

**Pending gate** (eyebrow `Pending gate`) — the one gate the desk can answer:

| Element | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Source | lock glyph + mono `machine · agent` | Where the gate came from | `🔒 macbook-pro · auth` |
| Gate title | `text-lg` semibold | The decision to make | `Approve database migration` |
| Exact command | mono, inverted (`bg-ink` on `paper`) block | The command shown *first* (approve-shape gate, R4) | `psql < 003_add_oauth.sql` |
| Relay note | `text-ink-soft` | Which phone got it + when + that the desk can also answer | `Relayed to iPhone 16 Pro 40s ago · awaiting your decision there, or answer here.` |
| Approve / Deny | primary (`accent`) + secondary, side-by-side | Resolve the gate from the desk | `Approve` · `Deny` |

Panel styling: the pending-gate card uses `accent-soft` background
with a `color-mix(... accent 30%)` border to read as the one thing
needing attention. A **decide-shape** gate (not shown in this
mockup's pending panel but per R4 / [[screen/relay-plan-author]])
replaces the command block with 3–4 option buttons plus a free-reply
field.

## Done gate

- The Relay rail item renders in the Observatory rail with the live
  `accent` dot and a pending-gate count that matches
  `pending_gates` from `GET /api/relay/status` (not a fabricated `1`).
- The four stat cards read from the same status payload:
  `pending gate`, `agents running`, `machines`, `phones paired` —
  colored accent / success / ink / ink per the mockup.
- The published stream lists real filtered events newest-first, each
  with `ts`, tone dot, `machine · kind` source line, and a safe
  detail summary. `live`-tone rows pulse; the gate row is tinted
  `accent-soft` and carries the `needs you` chip.
- **The redaction footer is always present**, naming
  `file contents · secrets · full diffs · shell output` — the
  zero-knowledge promise (R5) is stated on-screen, not implied.
- The pending-gate panel shows the source (`machine · agent`), the
  title, the **exact command first** for an approve gate, which
  phone it was relayed to and how long ago, and working
  Approve / Deny controls that POST the decision.
- Answering the gate here resolves the *same* gate the phone holds
  (idempotent); the row leaves the pending state and the rail count
  decrements.
- Pause publishing halts the outbound stream and the live badge
  reflects the paused state.
- The rail footer lists paired devices (machines + phones) with
  health dots, sourced from `devices`, not hardcoded.
- Dark mode: accent-soft tints, the inverted command block, and all
  tone dots stay legible.

## Wrong gate

- **A diff body, file contents, secrets, or shell output appears in
  a published event.** The stream must carry only the redacted
  summary line; anything more breaks the zero-knowledge contract
  (R5). The redaction footer would be a lie.
- **A gate is published but the pending panel / rail count is
  empty** (or vice-versa) — the stream, the pending list, and the
  rail badge disagree on the same gate.
- **The pending gate shows no command / no options.** An approve
  gate must surface the exact command first (R4); a decide gate must
  show its 3–4 options + free reply. A bare "approve?" with no
  context is wrong.
- **A gate has no source attribution** (`machine · agent`) — the
  user can't tell which machine or agent is asking.
- **The rail item shows a static `1`.** The badge must reflect the
  live pending-gate count.
- **Events from a machine other than this one appear as if this
  coordinator produced them.** Each coordinator supervises only its
  own machine; cross-machine events arrive via the relay grouped by
  their own machine, never re-attributed to this host.
- **The coordinator is shown as inbound-reachable** or the relay as
  able to read events — the relay is outbound-only and blind by
  construction.

## Related

- [[screen/relay-plan-author]] — marks which plan steps become the gates shown here
- [[screen/relay-dojo-gates]] — the team fan-out of these gates into a shared on-call queue
- [[architecture/relay]] — the three planes + the human-in-the-loop round-trip
- [[architecture/dojo]] — the shared queue the team variant routes into
- [[journeys/relay]] — run & supervise the round-trip
- [[screen/observatory-instruments-health]] — sibling Observatory rail item (format reference)
- [[pipeline/narration-cache]] — mentor-voice text for any generated status copy
