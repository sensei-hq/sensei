# 関 · Relay · Approve

**Segment:** 05 · Relay (mobile companion)
**Route:** Relay iOS app · approve gate (no web route — phone app)
**Source mockup:** [`lib/relay/relay.jsx`](../../mockups/Sensei/lib/relay/relay.jsx) → `RelayApprove`
**Data:** greenfield — an **approve gate** raised by the coordinator and delivered over the relay. Proposed: the gate payload `{ id, kind:'approve', machine, command, destructive: bool, reversible: bool, dry_run_available: bool }`; the reply `POST /api/relay/gates/{id}` `{ decision:'approve'|'reject', dry_run: bool }`. The command string is composed/echoed by the coordinator — it is the literal command, not a description.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + relay transport not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

An agent has hit a step that must stop for a human — and it's the kind of
step where the user has to know *exactly* what they're authorizing before
they tap. This full-screen gate exists to make that decision **exact and
unblindfolded**: the **exact command** is the hero, shown verbatim in a
high-contrast block, before any prose. The screen names the machine, flags
when the action is **destructive / not reversible**, and offers to **dry-run
in a sandbox first** so the user can approve safely. It is deliberately a
full takeover, not a toast — a destructive command deserves a full pause.
The one rule this screen enforces above all: you never approve something you
can't read.

## Data invariants

- The **exact command is mandatory and verbatim.** The coordinator sends the
  literal command string; the screen renders it unaltered in a monospace,
  high-contrast block (ink background, paper text in the mockup). There is
  **no approve path without an exact command** — a gate that arrives without
  one is malformed and must not render an Approve button.
- **ZERO-KNOWLEDGE holds here too:** the command string is *status* (what the
  agent intends to run) — but the surrounding code, the files it touches, and
  any transcript are not, and never cross the relay. The command is the single
  content-bearing string on the screen and it is coordinator-echoed, not raw
  agent stdout.
- `machine` names where the command will run (e.g. `mac-mini`) (R7).
- **Destructive/reversible flags** drive the warning. When `destructive`
  and/or `!reversible`, the screen shows the warning glyph and the
  `destructive · not reversible` sub-line. These are coordinator-set booleans,
  not inferred on the phone.
- **Dry-run toggle**: shown when `dry_run_available`. Default **on** for
  destructive gates (the mockup shows it on) — "Dry-run in a sandbox first".
  The toggle's state travels back with the decision so the coordinator runs
  the sandbox pass before the real command.
- Exactly two terminal actions: **Approve & run** (accent) and **Reject**
  (muted). A dismiss (×) closes without deciding (the gate stays pending).
- Approving sends `{decision:'approve', dry_run: <toggle>}`; the coordinator
  resumes the run (or runs the sandbox pass first, then re-gates if
  configured). Rejecting sends `{decision:'reject'}` and the coordinator
  halts that step.

## Signals shown

| Element | Value | Meaning | Example |
|---|---|---|---|
| Dismiss (×) | closes, gate stays pending | back out without deciding | top-right |
| Warning glyph | accent triangle in accent-soft tile | this is a stop-and-think gate | ⚠ |
| Headline | "The agent wants to run a command" (display) | why you're being pulled in | — |
| Machine + flags | `On {machine} · destructive · not reversible` | where + risk class | `On mac-mini · destructive · not reversible` |
| **Exact command** | verbatim mono block, ink bg / paper text | **the thing you're authorizing** (R4) | `rm -rf ./dist` / `npm run build --prod` |
| Dry-run toggle | on/off, default on for destructive | run in a sandbox first | `Dry-run in a sandbox first` — on |
| Approve & run | accent button | authorize, with dry-run per toggle | `Approve & run` |
| Reject | muted button | refuse the command | `Reject` |

Real mockup content: machine `mac-mini`; flags `destructive · not reversible`;
exact command block `rm -rf ./dist` then `npm run build --prod`; dry-run toggle
**on**; actions `Approve & run` / `Reject`.

## Done gate

The gate renders the coordinator's approve payload as a full-screen takeover
with the **exact command shown verbatim before any prose**, the target machine
named, destructive/not-reversible flagged with a warning, a dry-run-in-a-sandbox
toggle defaulting on for destructive gates, and exactly two terminal actions
(Approve & run / Reject). Approving sends the decision plus the dry-run state
back over the relay; nothing but the command string is content-bearing, and
even that is coordinator-echoed.

## Wrong gate

- **An Approve button with no exact command shown** (or with a paraphrase like
  "the agent wants to modify some files" instead of the literal command). This
  is the cardinal Relay failure (R4) — a gate with no exact command must not
  render Approve.
- **The command is editable, truncated, or reflowed** such that what's shown
  differs from what will run. The block must be verbatim; the user is
  authorizing exactly these characters.
- **Destructive gate defaults dry-run off** (or omits the toggle when a sandbox
  pass is available) — the safe default was dropped.
- **The warning/flags are inferred on the phone** from the command text rather
  than read from coordinator booleans — the phone must not guess destructiveness.
- **Any code, file contents, or transcript beyond the command string appears** —
  zero-knowledge violation (R5).
- **Approve resolves the gate but the dry-run choice is dropped**, so the
  coordinator runs the real command when the user asked for a sandbox pass first.

## Related

- Objectives [R4](../../objectives.md#relay--supervising-long-runs-from-anywhere) (minimal + exact human-in-the-loop · the **exact command**), [R5](../../objectives.md#relay--supervising-long-runs-from-anywhere) (zero-knowledge), [R7](../../objectives.md#relay--supervising-long-runs-from-anywhere) (which machine)
- [architecture/relay](../../architecture/relay.md) — gates come in two shapes: **approve** (exact command first) and **decide**
- [journeys/relay](../../journeys/relay.md) — `gate: approve → the exact command first`
- Sibling relay screens: [Task detail](relay-task-detail.md) (docked sheet that opens this) · [Respond](relay-respond.md) (the *decide* gate) · [Dashboard](relay-dashboard.md) · [Security](relay-security.md) · [Pairing](relay-pairing.md)
