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
