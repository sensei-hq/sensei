# Architecture

> **The HOW — at a design level.** How the layers realise the
> [requirements](../requirements/README.md). Start with this high-level view,
> then read the per-layer doc for the part you're changing. Buildable
> per-screen/per-pipeline contracts live in [`../spec/`](../spec/README.md);
> this folder owns component boundaries, data flow, and the rationale behind them.

Every layer doc opens by naming the [objectives](../requirements/objectives.md)
it serves, then describes structure and flow. Where a design decision pushes
against a [vision theme](../requirements/vision.md#the-six-non-negotiable-themes),
it says so.

## The system at a glance

```mermaid
flowchart TD
    subgraph client["User's machine (personal sensei — fully local)"]
        APP["app<br/>Tauri + SvelteKit desktop"]
        CLI["cli<br/>sensei binary"]
        ASSIST["AI assistants<br/>Claude Code · Zed · Cursor"]
        MKT["marketplace<br/>skills · commands · plugins · agents"]

        ASSIST -->|MCP stdio| MCP["mcp<br/>context server"]
        MKT -.->|hooks · plugin| ASSIST
        APP -->|HTTP :7744| D
        CLI -->|HTTP :7744| D
        MCP -->|HTTP :7744| D

        subgraph D["daemon (senseid) — the engine"]
            direction LR
            CAP[capture] --> GRAPH[code+activity graph]
            GRAPH --> ANALYZE[analyzer] --> LEARN[memories·patterns·recs]
            GW[["gateway<br/>(embedded LLM)"]]
            ANALYZE -.-> GW
            LEARN -.-> GW
        end
        D --> DATA[("data<br/>Postgres :7744 · sensei DB")]
        GW -.-> OLLAMA["ollama / embedded gguf"]
    end

    subgraph saas["Dōjō (opt-in SaaS — org boundary)"]
        CONSOLE["console<br/>maintainer · admin · lead web app"]
        DOJOSVC["dojo-mind service<br/>sensei-dojo"]
        CONSOLE --> DOJOSVC
    end

    D <-->|pull, never push · preview always| DOJOSVC
    WEB["website<br/>marketing + docs"]
```

## The layers

Each links to its detailed design. The order is data-up (foundation first).

| Layer | Doc | Owns | Serves objectives |
|---|---|---|---|
| **data** | [`data.md`](data.md) | DDL schema, the code+activity+inference+dojo models, DB conventions (dbd, port 7744) | all (the substrate) |
| **daemon** | [`daemon.md`](daemon.md) | `senseid` — task system, capture, scan, analyzer, the pipelines | B*, O*, P*, the core loop |
| **cli** | [`cli.md`](cli.md) | `sensei` binary — commands, config merge, hook script | B3, O4 |
| **app** | [`app.md`](app.md) | Tauri sidecar + SvelteKit UI (24-token rokkit, state layers) | F1–F3, O*, P* |
| **mcp** | [`mcp.md`](mcp.md) | context server — the tools an assistant calls mid-task | the core loop's *deliver* step |
| **marketplace** | [`marketplace.md`](marketplace.md) | skills · commands · plugins · agents (hooks, phase chains, mindsets) | capture + delivery into the assistant |
| **dojo** | [`dojo.md`](dojo.md) | the SaaS console + `dojo-mind` service — memberships, contribute/triage/distribute, anonymization | DJ1–DJ5, theme 5 |
| **website** | [`website.md`](website.md) | marketing site + docs surface | adoption |

## Cross-cutting concerns

- [`concepts/`](concepts/) — vocabulary the layers share: mindsets, personas,
  agents, governance.
- **Gateway** is an in-process LLM router consumed by the daemon as the
  `gateway-embedded` git dependency (sibling repo `sensei-hq/gateway`); it is no
  longer an in-tree crate. See [`daemon.md`](daemon.md#gateway).
- **Single mode.** One binary, one DB, port **7744** — no dev/prod split.

## Read next

- Foundation: [`data.md`](data.md) → [`daemon.md`](daemon.md).
- Where the gaps are: [`../requirements/open-issues.md`](../plan/README.md).
