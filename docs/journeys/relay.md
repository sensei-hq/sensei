# The relay journey

> Supervising long, multi-agent runs from your phone — the away-from-keyboard
> round-trip. Distilled from
> [`../mockups/Sensei/Sensei Relay.html`](../mockups/Sensei/Sensei%20Relay.html).
> Traces to [objectives R1–R8](../requirements/objectives.md#relay--supervising-long-runs-from-anywhere);
> the layer design is [architecture/relay](../architecture/relay.md).

## Onboarding — pair once

```mermaid
flowchart LR
    o1[Install the phone app] --> o2[Pairing round-trip<br/>encrypted keys with the coordinator]
    o2 --> o3[Grant permissions<br/>which projects · observe vs approve]
```

## Author a plan — make the run modular

```mermaid
flowchart LR
    p1[Project] --> p2[One active plan] --> p3[Phases n of x]
    p3 --> p4[Features · checkpoints] --> p5[Mark which steps GATE to a human]
```

## Run &amp; supervise — the round-trip

```mermaid
flowchart TD
    r1[Auto mode<br/>non-blocking] --> r2[Coordinator supervises the agent CLIs]
    r2 --> r3[Publishes filtered status<br/>done · doing · next · grouped by machine]
    r3 --> r4{Needs me?}
    r4 -->|track stalled| N[Nudge] --> r2
    r4 -->|gate: approve| A[Approve<br/>the exact command first] --> r2
    r4 -->|gate: decide| D[Decide<br/>3–4 options + a free reply] --> r2
    r4 -->|no| r3
```

Only *filtered status* crosses the **zero-knowledge relay** — code and
transcripts never leave your hardware.

## Team relay (Dōjō)

```mermaid
flowchart LR
    g1[A gate fires] --> g2[Shared on-call queue] --> g3[Whoever's on call decides]
    g3 --> g4[Attributed decision lands in the Dōjō record] --> g5[Coordinator resumes]
```

## The surfaces

**Phone:** Dashboard · Task detail (plan checklist + activity + gate) · Approve ·
Respond · Security · Pairing · Projects · Plan · Decisions · Nudge.
**Desktop (Observatory rail):** Coordinator (devices · published stream · pending
gate) · Plan authoring (mark gates).
**Dōjō:** shared gate queue with attribution.
