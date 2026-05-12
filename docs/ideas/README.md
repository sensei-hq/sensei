# Sensei

## What is Sensei

Sensei is a developer tool that observes AI-assisted coding sessions, learns patterns from your work, and coaches you toward better outcomes. It runs locally on your machine as a desktop app with a background daemon, connecting to your AI coding assistants (Claude Code, Cursor, Zed, Continue) without interrupting your workflow. Over time, it builds a memory of what works, what doesn't, and why -- then uses that knowledge to help you and your assistants produce higher-quality code on the first try.

---

## Core modules

| # | Module | Purpose |
|---|--------|---------|
| [01](./01-bootstrap.md) | **Bootstrap** | Get running: install dependencies, verify health, handle upgrades |
| [02](./02-setup.md) | **Setup** | Configure your workspace: 10-step wizard from welcome to done |
| [03](./03-observatory.md) | **Observatory** | Observe, measure, learn: daily dashboard, sessions, coaching, memory |
| [04](./04-project.md) | **Project** | Understand and work with your code: intelligence, patterns, search, traceability |
| [05](./05-gateway.md) | **Gateway** | Inference routing: model config, budget, providers, consensus |
| [06](./06-logging.md) | **Logging** | Diagnostics: log viewer, debug mode, issue submission |

---

## How Sensei works

Sensei is an invisible layer between you and your AI coding assistants. You never interact with Sensei during a coding session -- it watches and teaches.

**Hooks** capture events from your AI assistants. Every tool call, every correction, every file edit flows through to Sensei's daemon. The daemon indexes your code, computes analytics, detects patterns, and builds a memory of your projects. The desktop app surfaces these insights: what's going well, what's regressing, which patterns are emerging, and what the assistant should know next time.

The loop is simple: you code with your assistant, Sensei observes, and next session the assistant gets better context because Sensei learned from the last one.

---

## Key concepts

**FTR (First-Time-Right)** -- the hero metric. A session is FTR when the assistant produces correct code without corrections. Sensei tracks your FTR rate across projects and over time, surfacing what drives it up or down.

**Teachings** -- knowledge Sensei has learned from observing your sessions. A teaching starts as a suggestion, gains strength through reinforcement, and can mature into a rule that's automatically delivered to your assistant.

**Corrections** -- moments in a session where you had to fix what the assistant produced. Sensei clusters corrections to find recurring problems and generate teachings that prevent them.

**Sessions** -- a bounded unit of work with an AI assistant. Sensei tracks sessions end-to-end: what was attempted, how it went, how long it took, and whether it was first-time-right.

**Patterns** -- recurring code structures Sensei detects in your projects. Patterns can be positive (conventions to follow) or negative (anti-patterns to flag). They progress through a lifecycle: suggested, then gap, then rule.

**Context delivery** -- the mechanism by which Sensei feeds relevant knowledge to your AI assistant at the start of a session. This includes active memories, project patterns, recent corrections, and relevant library documentation -- ranked and budgeted to fit within token limits.

---

## Status

| Module | Status | Notes |
|--------|--------|-------|
| Bootstrap | Buggy | 6 gates implemented, needs stabilization |
| Setup | Buggy/Partial | Welcome-Roots done, Scan-Libraries partial |
| Setup: Instruments-Assignments | Not started | Needs gateway |
| Observatory | Not started | |
| Project | Partial | Indexing pipeline in progress |
| Gateway | Not started | Design complete |
| Logging | Not started | Design in progress |
