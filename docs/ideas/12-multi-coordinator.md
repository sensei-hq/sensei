---
name: Multi-Coordinator Support
description: Abstract sensei to work across AI coding tools — Claude Code, Cursor, Copilot, opencode, Kiro — via adapter pattern
date: 2026-04-17
status: idea
sources: features/04-multi-agent-support.md, roadmap/05-coordinator-adapters.md, gap-analysis.md
---

# Multi-Coordinator Support

## Problem

Sensei is currently hardcoded to Claude Code — hooks, skills, context delivery, and session protocols all assume Claude Code's plugin system. To reach broader adoption, sensei needs an abstraction layer that adapts to different AI coding coordinators.

## Current state

- Claude Code integration: implemented (plugin, hooks, MCP, skills)
- Coordinator adapter pattern: designed in docs, not implemented
- Skill format transformation: planned (canonical → coordinator-specific at install time)
- Event capture for non-Claude tools: not designed
- Multi-coordinator context delivery: not implemented

## What this idea covers

- **CoordinatorAdapter interface**: abstract hooks, skills, context files, and session protocols per coordinator
- **Supported coordinators**: Claude Code, Cursor, GitHub Copilot, opencode, Kiro (initial targets)
- **Skill transformation**: canonical skill format that gets compiled to coordinator-specific formats
- **Event capture fallbacks**: hooks for Claude Code; file-watching/polling for others; OTLP for standardized telemetry
- **Context file generation**: CLAUDE.md, .cursorrules, copilot-instructions.md generated from a single source

## Open questions

- Which coordinator to support second after Claude Code? Cursor has the largest user base.
- How much of the workflow system (phases, commands) is Claude Code-specific vs. portable?
- Is MCP the universal transport, or do some coordinators need different integration points?
