# Sensei Roadmap

> **Direction as of 2026-04-08** — This folder supersedes the prior SaaS platform direction
> (`docs/design/24-platform-architecture.md`, now archived) and the interim self-hosted model
> (`docs/product/self-hosted-model.md`). Start here.

---

## The One-Line Shift

Sensei was a Claude Code companion that helped with code indexing and context.
Sensei becomes a **lifecycle guide** — an executive assistant that meets developers at any point
in their thinking, from raw idea to shipped code, and helps them be more deliberate, more efficient,
and more effective with AI.

---

## Document Index

| Document | What it covers |
|---|---|
| [01-paradigm-shift](./01-paradigm-shift.md) | Why the current model falls short, the insight behind the change, what stays and what goes |
| [02-local-architecture](./02-local-architecture.md) | New architecture: Tauri desktop app, SQLite, graph model, two-binary split, local inference |
| [03-workspace-model](./03-workspace-model.md) | How projects, ideas, phases, and cards are organised — flexible by design |
| [04-implementation-plan](./04-implementation-plan.md) | Phased build plan: what to build, in what order, and why |
| [05-coordinator-adapters](./05-coordinator-adapters.md) | ACP vs. model abstraction; Claude Code as default; ACPAdapter interface and ACPRegistry |
| [06-graph-intelligence](./06-graph-intelligence.md) | Analysis of Graphify and Karpathy's wiki model; what sensei should adopt and how |
| [07-gaps](./07-gaps.md) | Open decisions and blockers — must resolve before implementation starts |
| [08-sqlite-schema](./08-sqlite-schema.md) | SQLite schema design: two databases, type mapping, WAL config, query function migration |

---

## Status Summary

| Phase | Description | Status |
|---|---|---|
| 0 — Simplify | Replace Supabase/pgvector with Kuzu (graph) + SQLite-vec, remove Docker dependency | Not started |
| 1 — Tauri shell | Wrap existing SvelteKit app in Tauri, two-binary model | Not started |
| 2 — Workspace model | Project/idea registry with maturity states | Not started |
| 3 — Card system | Cards, phases (optional), links between cards and code | Not started |
| 4 — Prompt workspace | Context-aware prompt bar, citations, built-in commands | Not started |
| 5 — Local inference | Ollama/Gemma for indexing description generation (optional) | Not started |
| 6 — Distribution | Tauri installer, auto-update, donation integration | Not started |

---

## Design Principles

- **ACP-agnostic** — Claude Code is the default, not the only option. opencode,
  GitHub Copilot, Kiro CLI, OpenAI Codex, and others are supported via `ACPAdapter`.
  ACP = Agent-Computer Protocol (Zed's framing). See `05-coordinator-adapters.md`.
- **Model-agnostic** — Claude is the default model. Any OpenAI-compatible endpoint works
  via the existing `ModelBackend` interface.
- **Local-first** — no cloud dependencies, no Docker, no accounts.
- **Flexible, not forced** — phases and cards are available, never required.

## What Is Not Changing

The core engine is sound and remains:

- Symbol indexing (L0–L5 resolution levels, Kuzu graph DB + tree-sitter)
- MCP server (protocol-standard, works with any MCP-capable ACP)
- Session continuity and FTR tracking
- Library intelligence
- The `packages/engine`, `packages/server`, `packages/collector` packages

The direction changes. The foundation stays.
