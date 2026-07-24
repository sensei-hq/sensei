# 群 · Relay · Dōjō shared gates

**Segment:** 05 · Relay (Dōjō console) — the team fan-out of relay gates
**Route:** `dojo.sensei-hq.org/{org}/console/relay-gates` (Dōjō console, SaaS) OR the self-hosted equivalent — **not** the desktop app
**Source mockup:** [`lib/relay/relay-desktop.jsx`](../../mockups/Sensei/lib/relay/relay-desktop.jsx) → `DojoRelayGates`
**Data:** _greenfield — org-wide relay gates fan into one shared on-call queue; each gate carries source attribution + a handler; decisions land in the Dōjō record; shapes proposed below_
**App file:** _greenfield — not built_ (Dōjō `dojo/`, a SvelteKit route, SSO-gated — see [architecture/dojo](../../architecture/dojo.md))
**Backend files:** _greenfield — team relay routing not built_ (grows the dojo Worker's `/v1`; the coordinators stay **outbound-only** and publish gates into the Dōjō, which fans them to on-call)
**Status:** greenfield (Relay + team routing unbuilt) — see [architecture/relay](../../architecture/relay.md) "Team relay (Dōjō)", objective [R8](../../objectives.md#relay--supervising-long-runs-from-anywhere)

## Purpose

When agents run for the whole org, their human-in-the-loop moments
**fan into one shared queue**. A gate raised on any developer's
machine (or a CI runner) can be **routed to whoever is on call**;
every decision **carries attribution** and **lands back in the Dōjō
record**. This is the team variant of the [[screen/relay-coordinator]]
pending-gate — instead of one desk answering one gate, the org's gates
collect into a single console the on-call engineer works.

This screen lives in the **Dōjō console** (the SaaS/self-hosted
maintainer surface), not the desktop app. It reuses the Dōjō's
sessions/projects/machines vocabulary and its attribution + audit-trail
mechanics. It delivers **R8 (team relay routes to on-call)** and
respects the Dōjō principle that **nothing travels unseen** — a gate
still carries only *filtered status*, never code; the on-call engineer
decides on the decision + its context, exactly as the coordinator
published it.

Kanji is 群 — *the crowd / the team* (the Dōjō angle: many people, one queue).

## Data invariants

The org's coordinators stay outbound-only; they publish gates into the
Dōjō, which fans them into one shared queue with attribution.

- **One shared queue across the org.** Gates from every member's
  machine and every CI runner collect here, ranked/ordered by
  urgency and age. The queue is scoped to the viewer's Dōjō
  membership (org boundary — theme 5).
- **Each gate carries source attribution end-to-end:** the
  originating `machine · agent` (e.g. `macbook-pro · Aiko`,
  `ci-runner · Codex`, `mac-mini · Claude`, `bastion · Aider`), the
  `project` (`lumen-auth`, `billing-svc`, `infra`), and — once
  claimed — the **handler** (`Aiko N.`, `Rai T.`, `Mei L.`). An
  unclaimed gate shows `— unclaimed`.
- **A gate has a status lifecycle** (`open → routed → oncall →
  approved`/denied). `open` = in the queue, unclaimed; `routed` =
  assigned to a person; `oncall` = the current on-call is handling
  it; `approved`/denied = resolved (a `success`-toned terminal
  state). The mockup's four rows show `oncall`, `routed`, `open`,
  `approved`.
- **Route-to-on-call is explicit.** A gate can be routed to whoever
  is on call (or a named person); routing records who it went to and
  when. On-call is a Dōjō role/rotation, not ad-hoc.
- **Every decision is attributed and durable.** When a gate is
  answered, the decision + the handler + the timestamp land in the
  **Dōjō record** (audit trail). Attribution is *not optional* — a
  resolved gate always names who decided (DJ4: attribution +
  confidentiality are automatic).
- **Still zero-knowledge.** The queue carries the same *filtered
  status* the coordinator published — the gate's decision and its
  context (title, source, project), never code or transcripts. The
  Dōjō relays the encrypted blob; routing to on-call happens on
  filtered metadata (event kind, machine, project), not on content.
- **The org boundary holds.** A client-engagement gate is subject to
  the same anonymization / preview the Dōjō applies to any client
  lesson before it is visible beyond the engagement (DJ2 — deferred
  with the rest of Dōjō live activation, but the invariant stands).

Proposed shapes (greenfield — name the contract, don't fabricate values):

```
GET /api/dojo/relay/gates?org=… →
{
  "summary": { "awaiting": 3, "resolved_today": 12, "median_to_answer_s": 84 },
  "gates": [
    { "gate_id": "…", "title": "Approve prod migration", "project": "lumen-auth",
      "source": "macbook-pro · Aiko", "handled_by": "Aiko N.", "status": "oncall" },
    { "gate_id": "…", "title": "Merge to main",          "project": "billing-svc",
      "source": "ci-runner · Codex",  "handled_by": "Rai T.", "status": "routed" },
    { "gate_id": "…", "title": "Re-auth Anthropic limit","project": "lumen-auth",
      "source": "mac-mini · Claude",  "handled_by": null,      "status": "open" },
    { "gate_id": "…", "title": "Rotate staging secret",  "project": "infra",
      "source": "bastion · Aider",    "handled_by": "Mei L.", "status": "approved" }
  ]
}

POST /api/dojo/relay/gates/{gate_id}/route   { "to": "oncall" | "member:{id}" }
POST /api/dojo/relay/gates/{gate_id}/decide  { "decision": "approve" | "deny", "reply"?: string }
   → resolves the gate, writes the attributed decision into the Dōjō record
```

## Signals shown

Header — eyebrow `群 · dōjō · shared relay`, title **"Gates across the
team"**, sub-copy: *"When agents run for the whole org, their
human-in-the-loop moments fan into one shared queue. Route a gate to
whoever's on call; every decision carries attribution and lands back
in the Dōjō record."* Right side: an open-count badge (`● 3 open`).

Summary stats (3-up):

| Element | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Awaiting | count, `accent` | Gates not yet resolved | `3 awaiting` |
| Resolved today | count, `ink` | Gates answered today | `12 resolved today` |
| Median to answer | duration, `success` | Team responsiveness | `1.4m median to answer` |

Queue table — a 5-column grid (`Gate · Project · Source · Handled by · Status`):

| Column | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Gate | lock glyph + `text-sm` medium label | The decision to make (lock tinted accent, or success when resolved) | `🔒 Approve prod migration` · `🔒 Merge to main` |
| Project | mono `text-xs text-ink-mute` | The org project the gate belongs to | `lumen-auth` · `billing-svc` · `infra` |
| Source | mono `text-xs text-ink-mute` | Originating `machine · agent` — the attribution of *where* | `macbook-pro · Aiko` · `ci-runner · Codex` · `mac-mini · Claude` · `bastion · Aider` |
| Handled by | `text-sm text-ink-soft`, or `— unclaimed` faint | Who claimed/decided it (attribution of *who*) | `Aiko N.` · `Rai T.` · `— unclaimed` · `Mei L.` |
| Status | `zs-badge`, capitalized; accent open / success resolved | Lifecycle state | `oncall` · `routed` · `open` · `approved` |

Row / status coloring: unresolved gates use `accent` tones (lock +
status badge); a resolved gate (`approved`) flips the lock + badge to
`success`. The `— unclaimed` handler renders in `ink-faint` to draw
the on-call to pick it up.

Not in this mockup but implied by R8 / the Dōjō record (note for the
detail view): clicking a gate opens the decision surface (route,
approve/deny + reply) and, once resolved, the audit entry showing the
attributed decision in the Dōjō record.

## Done gate

- The queue renders one row per gate from
  `GET /api/dojo/relay/gates`, each with its title (lock-prefixed),
  project, `machine · agent` source, handler, and status badge —
  across the five columns.
- Source attribution (`machine · agent`) is present on **every** row
  — the queue always shows *where* each gate came from, across
  Claude · Codex · Aider · and CI runners (R7/R8).
- Unclaimed gates render `— unclaimed` in a muted style; claimed/
  resolved gates name the handler.
- Status badges are accent-toned for open/routed/oncall and
  success-toned for resolved (`approved`); the resolved lock glyph
  also flips to success.
- The 3 summary stats (`awaiting` accent, `resolved today` ink,
  `median to answer` success) read from the summary payload, and the
  header open-count badge matches `summary.awaiting`.
- Routing a gate to on-call assigns it and records who + when;
  answering it writes an **attributed** decision into the Dōjō
  record (the handler + timestamp are durable, not discarded).
- The queue is scoped to the viewer's Dōjō membership (org
  boundary); no cross-org gate leaks in.
- The gate carries only filtered status (title, source, project) —
  no code or transcript surfaces in the console.
- Dark mode: accent vs. success rows stay distinguishable; the
  `— unclaimed` muted state remains readable.

## Wrong gate

- **A gate is routed to nobody / has no on-call target** — a fanned
  gate that lands in a queue with no route and no owner is a
  dropped human-in-the-loop moment (breaks R8).
- **Attribution is missing on a resolved decision** — a gate is
  marked `approved` but the Dōjō record doesn't name who decided.
  Every team decision must carry attribution (DJ4); an anonymous
  approval is wrong.
- **Source attribution is missing or generic** — a gate shows no
  `machine · agent`, so the on-call can't tell which machine/agent/
  developer raised it.
- **Code or a transcript is published to the queue** — the console
  must show only filtered status; anything a coordinator would have
  redacted must not appear here either (R5, DJ2).
- **A cross-org / client-engagement gate leaks into another
  membership's queue** without the Dōjō's anonymization + preview —
  the org boundary must be exact (theme 5 / DJ2).
- **The queue mixes resolved and open with no lifecycle distinction**
  — resolved gates must be visually terminal (success tone), not
  competing for the on-call's attention.
- **Summary stats disagree with the queue** — `awaiting` doesn't
  match the count of unresolved rows, or the header badge disagrees
  with `summary.awaiting`.
- **The console is treated as inbound-reaching the coordinators** —
  coordinators stay outbound-only; the Dōjō fans out gates and
  carries replies back through the relay, never by reaching into a
  machine.

## Related

- [[screen/relay-coordinator]] — the single-desk pending gate this screen fans across a team
- [[screen/relay-plan-author]] — where these gates were marked on a plan
- [[architecture/relay]] — "Team relay (Dōjō)" — gates fan into one shared on-call queue
- [[architecture/dojo]] — the shared queue's home; roles, attribution, audit trail
- [[journeys/relay]] — "team relay (Dōjō)" round-trip
- [[screen/dojo-maintainer-console]] — sibling Dōjō console (queue + attribution format reference)
- [[pipeline/dojo-lifecycle]] — how an attributed decision lands in the Dōjō record
