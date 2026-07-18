# The personal journey

> How one developer + their assistant move through sensei. Distilled from
> [`../mockups/Sensei/Sensei Journey Map.html`](../mockups/Sensei/Sensei%20Journey%20Map.html).
> Traces to [requirements/objectives](../requirements/objectives.md). The
> end-to-end version — Rin, working across several orgs — is in
> [`../mockups/Sensei/Sensei End-to-End Journey.html`](../mockups/Sensei/Sensei%20End-to-End%20Journey.html)
> (see [journeys → the whole loop](README.md#the-whole-loop--machine--dōjō--beyond)).

## The core loop — everything is one loop

```mermaid
flowchart LR
    A[capture<br/>sessions · tool calls · prompts · outcomes] --> B[graph<br/>code + activity over real repos]
    B --> C[analyze<br/>enrich · signals · patterns · FTR]
    C --> D[learn<br/>memories · patterns · guards · recommendations]
    D --> E[deliver context<br/>MCP → the assistant, first try]
    E --> F{{FTR ↑?}}
    F -->|measure verdict| C
    E -.->|the next session| A
```

Each layer of the [architecture](../architecture/README.md) exists to keep this
turning. The north-star, **FTR** (first-turn resolution), is the loop's scorecard.

## The four segments

```mermaid
flowchart TD
    subgraph S1["01 · Bootstrap 支"]
        direction LR
        b1[verify what's there] --> b2[bring the toolchain up green]
    end
    subgraph S2["02 · First run &amp; Preferences 名"]
        direction LR
        f1[point at real folders] --> f2[projects appear] --> f3[tune the defaults]
    end
    subgraph S3["03 · Observatory — daily use 家"]
        direction LR
        o1[today's one thing] --> o2[act on it] --> o3[stay in control of what leaves]
    end
    subgraph S4["04 · The project window 雲"]
        direction LR
        p1[work inside one project] --> p2[trust what it learned] --> p3[before any of it travels]
    end
    S1 --> S2 --> S3 --> S4
    D[["Dōjō — cross-cutting SaaS layer<br/>(threads through 03 + 04)"]]
    S3 -.-> D
    S4 -.-> D
```

**Value before setup** is literal: the first thing the user does is see *their
own projects* (segment 02), not a wizard. The old nine-stage wizard is gone —
tuning lives in Preferences, reachable but never blocking.

## The loops inside the daily app

The Observatory and project window aren't flat screens — each domain is its own
small retrospective loop (mockup: *"Module lifecycles"*). Each **observes →
forms a finding → offers one action** (Apply · Review · Dismiss):

```mermaid
flowchart LR
    subgraph modules["Module lifecycles"]
        direction LR
        m1[Security &amp; guards] ~~~ m2[Architecture] ~~~ m3[Testing] ~~~ m4[Style &amp; conventions]
        m5[Memory] ~~~ m6[Traceability] ~~~ m7[Impact] ~~~ m8[Libraries] ~~~ m9[Insights]
    end
    obs[observe] --> find[form a finding] --> act[offer one action]
```

## Where Dōjō threads in

At three points the personal journey touches the team layer — always opt-in,
always previewed: **bind** a project to an org (segment 04 · About), a **finding
forms** and is shared upstream (04 · Memories/Patterns), and approved knowledge
is **distributed back** (03 · Today/Upgrades). The full team path is in
[`dojo.md`](dojo.md).
