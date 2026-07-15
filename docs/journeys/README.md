# Journeys

> **The visual perspective.** The flows a human walks through sensei — drawn as
> diagrams so the shape is obvious at a glance. Journeys sit between the
> [requirements](../requirements/README.md) (what/why) and the
> [mockups](../mockups/) (pixel-level screens): they show *how the segments
> connect into a path*.

## Contents

- [`sensei.md`](sensei.md) — the personal journey: the core retrospective loop,
  the four segments (Bootstrap → First-run → Observatory → Project window), and
  the module lifecycles inside the daily app.
- [`dojo.md`](dojo.md) — the team journey: the Dōjō roles
  (developer · maintainer · admin · lead) and the contribute→distribute
  lifecycle, plus **Relay** (folded in — reach a live session through the Dōjō),
  the confidentiality principles, and the business model.
- [`relay.md`](relay.md) — **folded into the Dōjō** (a pointer). Relay is no
  longer a separate pair-once app; it reaches a live session *through* the Dōjō
  over realtime. See [`dojo.md` → Relay](dojo.md#relay--away-from-keyboard-through-the-dōjō).

## Visual sources of truth (mockups)

The diagrams here distill these interactive mockups — open them for the full
narrative:

- [`../mockups/Sensei/Sensei Journey Map.html`](../mockups/Sensei/Sensei%20Journey%20Map.html) — the personal journey + module lifecycles.
- [`../mockups/Sensei/Sensei Dōjō Journey Map.html`](../mockups/Sensei/Sensei%20D%C5%8Djo%20Journey%20Map.html) — the four Dōjō role journeys.
- [`../mockups/Sensei/Sensei Flow Walkthrough.html`](../mockups/Sensei/Sensei%20Flow%20Walkthrough.html) — a step-by-step click-through of the app.

## Reading position

```mermaid
flowchart LR
    R[requirements/<br/>what &amp; why] --> J[journeys/<br/>the path] --> M[mockups/<br/>the screens]
    M --> A[architecture/<br/>how] --> S[spec/<br/>buildable] --> P[plan/<br/>what's next]
```
