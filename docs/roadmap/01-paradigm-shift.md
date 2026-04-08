# Paradigm Shift — From Code Tool to Lifecycle Guide

---

## What Sensei Was

A **Claude Code companion** focused on the implementation phase:

- Index a codebase once, serve ranked context to the agent
- Track session continuity and FTR scores
- Manage library documentation and drift detection
- Reduce token waste by giving agents the right symbols at the right resolution

This is valuable. It is also incomplete.

---

## The Problem It Did Not Solve

Claude is capable of helping at every stage of software development:
requirements gathering, analysis, design thinking, gap identification, architectural
review, and implementation. Most developers use it only for the last one.

The result is a predictable pattern:

```
Vague requirement
     ↓
Jump to implementation
     ↓
Rework because the design was wrong
     ↓
Rework because a requirement was missed
     ↓
Rework because an existing pattern was not considered
     ↓
Ship
```

Each rework cycle is expensive — not just in time, but in tokens and cognitive load.
The AI is being used as a fast typist rather than as a thinking partner.

**The bottleneck is not the AI's capability. It is the developer's workflow.**

---

## The Insight

Developers who invest 20 minutes in upstream thinking — articulating the problem,
exploring the space, sketching a design — consistently produce better implementations
in fewer cycles than developers who go straight to code.

Sensei's job is to make that upstream thinking low-friction, connected to the codebase,
and usable as context for the implementation phase that follows.

This is not about adding process overhead. It is about making the natural thinking that
good developers already do *visible, persistent, and reusable*.

---

## The Mental Model Change

| Before | After |
|---|---|
| Sensei is a tool you configure once per repo | Sensei is a workspace you return to regularly |
| Primary surface: terminal (`sensei` CLI) | Primary surface: desktop app with prompt-first UI |
| Organises around: files and symbols | Organises around: projects, ideas, and phases of thinking |
| Session = a coding session | Session = a thinking + coding session |
| Context = code slices | Context = code + design decisions + requirements + notes |
| Output = better code | Output = better thinking → better code |

---

## Sensei as Executive Assistant

The frame that best describes the new role: **executive assistant and technical mentor**.

An executive assistant does not do the work for you. They:
- Know what you are working on and what matters
- Surface the right information at the right moment
- Track decisions and commitments so you do not have to remember them
- Flag when something is inconsistent or missing
- Help you think out loud by asking clarifying questions
- Keep the broader picture visible while you are in the details

Applied to software development:

- You are exploring an idea → Sensei helps you articulate it, find related prior work, ask what you have not considered
- You are writing requirements → Sensei checks for ambiguity, gaps, conflicts with existing capabilities
- You are designing → Sensei shows you what already exists, flags patterns, surfaces relevant ADRs
- You are implementing → Sensei gives ranked context, tracks what changed, measures quality
- You review → Sensei links changes back to requirements, shows traceability, scores first-time-right

At every stage, you can prompt naturally. The system cites its sources.

---

## What Changes

### Out: Supabase as the data layer

Supabase is an excellent platform for SaaS. For a personal developer tool, it is
infrastructure the user should not have to manage. Supabase is replaced by embedded SQLite.
No Docker. No cloud credentials. The data lives in a local file the developer owns.

### Out: Cloud dashboard, auth, org accounts

No login. No accounts. No billing. The dashboard is a local view of local data.
Donation-based sustainability (Buy Me Coffee / GitHub Sponsors) rather than SaaS pricing.

### Out: Rigid, mandatory pipeline

The current model implicitly assumes: init → index → work. The new model has no mandatory
order. A developer can start with a raw idea note, or jump straight to implementation and
add context later, or do requirements first. The tool adapts to how the developer works.

### In: Tauri desktop app

A proper desktop app with a native installer, file system access, and a webview embedding
the SvelteKit frontend. No browser tab, no localhost port to remember.

### In: Graph-first data model

Everything (files, symbols, cards, requirements, decisions, libraries) is a node in a graph.
Edges capture relationships: file exports symbol, requirement drives design, design informs task,
task modifies symbol. This makes traceability first-class without requiring the developer to
manually maintain it.

### In: Local inference (optional)

Ollama + Gemma handles description generation during indexing. No cloud API call required
for the indexing phase. Claude (via Claude Code) remains the primary reasoning partner
for the thinking and implementation phases.

### In: Prompt-first workspace

The primary interaction model across all phases. Ask a question, get an answer with citations.
Run a built-in command, get structured output. Everything is interactable via prompt —
the phase structure, the cards, the code, the decisions.

---

## What Does Not Change

- The engine: symbol extraction, L0–L3 resolution, ranking, context packs
- The MCP server: Claude Code still connects to sensei for context and session continuity
- The collector daemon: session tracking, FTR scoring
- The library intelligence: indexing third-party and internal libraries
- The skill system: per-agent skill files, hook installation
- The TypeScript/Bun monorepo structure

The product direction changes. The technical foundation is preserved and extended.

---

## Prior Documentation Status

| Document | Status after this shift |
|---|---|
| `docs/design/24-platform-architecture.md` | Archived — SaaS model deferred |
| `docs/product/self-hosted-model.md` | Superseded by this roadmap |
| `docs/product/saas-model.md` | Archived |
| `docs/design/01-architecture.md` | Partially valid — Supabase references obsolete |
| `docs/features/09-identity-access-pricing.md` | Obsolete — no identity layer in new model |
| All other design and feature docs | Valid for core engine behaviour |
