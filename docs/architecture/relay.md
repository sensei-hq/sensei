# Layer · relay → folded into the Dōjō

> **Relay is now part of the Dōjō layer.** As of the unified Dōjō+Relay revision
> (2026-07-15), Relay is no longer a separate transport you pair — the daemon's
> existing outbound Dōjō connection carries live session control over **Supabase
> realtime**, and any signed-in phone or console reaches a running session
> through it.

The relay architecture now lives in the Dōjō layer:

- **Architecture:** [`dojo.md` → Relay — through the Dōjō](dojo.md#relay--through-the-dōjō)
  — the realtime line (daemon stays outbound-only, zero-knowledge), the responsive
  PWA + Web Push + thin Capacitor wrapper, and the relay data model
  (`push_subscriptions` · push dispatch · `relay_sessions` + presence ·
  `relay_inbox` · `notification_prefs` · the bidirectional daemon↔Dōjō channel).
- **Journey:** [`../journeys/dojo.md` → Relay](../journeys/dojo.md#relay--away-from-keyboard-through-the-dōjō).

### What changed from the prior model

The old design (see git history of this file) used a **standalone zero-knowledge
relay transport** with **device pairing** and a separate mobile app. That is
superseded:

- **No pairing / no separate transport** — reuse the daemon's outbound Dōjō line
  over realtime. No NAT traversal, no inbound ports.
- **No separate app to install** — one responsive **PWA**; a thin native wrapper
  exists only for reliable **push + offline** (iOS APNs / Android FCM).
- **Same guarantees kept** — outbound-only, zero-knowledge (only filtered status +
  gate prompts + replies cross), scoped/revocable, attributed team decisions.

The **planner model** (project → plan → phase → {feature · checkpoint · gate},
non-blocking auto mode, approve/decide gates) is unchanged and belongs with the
[daemon](daemon.md) coordinator + the [app](app.md) plan-authoring surface.

## Local crews — the autonomous queue (2026-11-12)

Beside the Dōjō line, the daemon brokers **local crews of assistant sessions** (the lanes model,
[`../prd/modules/03-delivery/21-relay/lanes.md`](../prd/modules/03-delivery/21-relay/lanes.md),
data model in [`lanes-data.md`](../prd/modules/03-delivery/21-relay/lanes-data.md)). Work is a queue, never a
hand-off: the lane loop is `relay_claim_next` → work → `relay_checkpoint_task` → `relay_report_task` (the report
floats for a peer) → `relay_submit_review`, and the broker's deterministic gate (`crates/senseid/src/relay/gate.rs`)
merges what a peer approved. The person's loop is register lanes → **Create sprint** on the Relay screen
(`POST /relay/runs`; Task 0 plans, a peer approves, the plan unpacks) → watch the band. The sweeper
(`crates/senseid/src/tasks/relay_lease_sweep.rs`) expires leases, re-offers, ends pauses, releases abandoned review
claims, nudges about floating reviews and runs the gate. Design and deviations:
[`../design/relay-autonomous-queue-and-planning.md`](../design/relay-autonomous-queue-and-planning.md),
plan [`../plan/2026-11-10-relay-autonomous-queue/README.md`](../plan/2026-11-10-relay-autonomous-queue/README.md).
