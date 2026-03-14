# Features

## Objective

Sensei makes AI coding agents more effective in software development by solving the orientation, context, continuity, and quality measurement problems that make unassisted agents slow and expensive. An agent with sensei orients in one tool call, loads exactly the context it needs, delegates deterministic work to MCP tools, carries knowledge forward across sessions, and measures its own improvement over time — spending tokens on reasoning, not rediscovery.

## Vision

- **Index Once, Orient Fast** — Scan a repo once and produce structured artifacts. Future agents orient in a single call instead of reading dozens of files.
- **Right Resolution for the Task** — Code has four representations. Agents load signatures for discovery, logic flows for understanding, and full source only when editing. Never more than needed.
- **Libraries Are First-Class** — Custom and third-party libraries are indexed, cached, and summarised into skills. Agents work from local knowledge rather than generic training data or expensive doc crawls.
- **Docs Stay in Sync** — Design docs, code, and public documentation are tracked together. Drift is detected automatically and reported before it becomes a problem.
- **Deterministic Work Belongs in Tools** — Generating llms.txt, checking drift, listing exports, fetching doc pages — these are repeatable, deterministic tasks. MCP tools handle them so the LLM's context stays clear for judgment.
- **Context Is Managed, Not Accumulated** — Agents load targeted slices, checkpoint before switching tasks, and never carry stale context into new work.
- **Quality Is Measured, Not Assumed** — First-Time-Right scores, benchmark comparisons, and gap reports quantify the real impact of skills and surface where agents are underperforming.
- **One Command to Get Started** — Setup takes seconds. Model config, ranking strategies, and agent adapters adapt to your team without manual wiring.

## Modules

| # | Module | Goal | File |
|---|--------|------|------|
| 01 | Codebase Intelligence | Sensei understands your repo so your agent doesn't have to explore it | [01-codebase-intelligence.md](01-codebase-intelligence.md) |
| 02 | Smart Context Delivery | Your agent gets exactly what it needs for the current task, nothing more | [02-smart-context-delivery.md](02-smart-context-delivery.md) |
| 03 | Session Continuity | Sensei remembers where you were and picks up from there | [03-session-continuity.md](03-session-continuity.md) |
| 04 | Multi-Agent Support | Sensei works with Claude, Cursor, opencode, and any agent you use | [04-multi-agent-support.md](04-multi-agent-support.md) |
| 05 | Library & Documentation Intelligence | Sensei knows about the libraries you use — internal and external | [05-library-intelligence.md](05-library-intelligence.md) |
| 06 | Documentation & Traceability | Sensei keeps design docs, code, and specs in sync | [06-documentation-traceability.md](06-documentation-traceability.md) |
| 07 | Analytics | Sensei measures quality, attribution, and improvement — and makes it visible | [07-analytics.md](07-analytics.md) |
| 08 | Setup & Configuration | Sensei is one command to get started and adapts to your team | [08-setup-configuration.md](08-setup-configuration.md) |
| 09 | Identity, Access & Pricing | Sensei is free for open source, affordable for teams, and your code never leaves your control | [09-identity-access-pricing.md](09-identity-access-pricing.md) |
| 10 | System Intelligence | Sensei understands your entire system — not just individual repos — and surfaces gaps where reality deviates from design | [10-system-intelligence.md](10-system-intelligence.md) |

## Traceability

Feature items and their status are tracked in sensei's traceability store. The generated export for CI and offline use is not the source of truth — it is a snapshot. Snapshots can be generated on demand and filtered by sprint.
