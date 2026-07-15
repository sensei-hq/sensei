# The relay journey → folded into the Dōjō

> **Relay is now part of the Dōjō.** As of the unified Dōjō+Relay revision
> (2026-07-15), Relay is no longer a separate app you pair — the daemon holds a
> live line to the Dōjō (Supabase realtime), and any signed-in phone or console
> reaches a running session **through the Dōjō**, no pairing.

The relay journey now lives as a section of the Dōjō journey:

- **Journey:** [`dojo.md` → Relay](dojo.md#relay--away-from-keyboard-through-the-dōjō)
  — reach a live session · watch progress · approve · answer a decision · chat back.
- **Architecture:** [`../architecture/dojo.md` → Relay](../architecture/dojo.md#relay--through-the-dōjō)
  — the realtime line, PWA + Web Push, the native wrapper, and the relay data model.
- **Business model:** Relay is [free for individuals, paid where shared](dojo.md#business-model--free-where-public-or-personal).

The prior standalone "pair once + zero-knowledge relay transport" model is
superseded: the transport is now the daemon's existing outbound Dōjō connection,
reused for realtime. Only *filtered status* + gate prompts + replies cross —
never code or transcripts.
