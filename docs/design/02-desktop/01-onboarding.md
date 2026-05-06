# 01 — Onboarding

> Route: `/setup` — shown on first launch or when no projects are configured.

## Purpose

Get the developer from zero to a working observatory in under 5 minutes. Scan their machine, discover repos, create projects, and configure AI coding platforms to send data through sensei.

This is the only time the app asks the user to do significant setup work. Everything after this is incremental.

## Flow

### Step 1: Welcome

Brief explanation of what sensei does. One button: "Get Started".

No login, no account creation, no telemetry consent. The daemon runs locally, data stays on machine.

### Step 2: Configure ACPs

Detect which AI coding platforms are installed (Claude Code, Cursor, Windsurf, Zed, Kiro, OpenCode, VS Code).

For each detected platform:
- Show detection status (installed / not found)
- Toggle to configure MCP registration
- For Claude Code: install the sensei plugin from marketplace (handles commands, skills, hooks, MCP in one step)
- For others: register MCP server entry in their config file

**Why this matters:** Without MCP configured, the AI can't call sensei tools. Without hooks configured (Claude Code plugin), sessions aren't captured. This step determines what data flows into the observatory.

### Step 3: Scan Folders

User picks one or more parent directories to scan (e.g., `~/Developer`, `~/projects`).

Uses native folder picker (Tauri). Multiple roots allowed. The daemon scans recursively for git repos.

### Step 4: Discover Repos

Live polling while the daemon scans. Show discovered repos as they appear with name, path, language detection.

Progress indicator: "Discovering repos... found 23 repos"

### Step 5: Group into Projects

Auto-detection suggests groupings:
- **Monorepos** (workspace config detected) → auto-create project
- **Name-prefix clusters** (`acme-api`, `acme-ui`, `acme-shared`) → suggest project "Acme"
- **Parent folder** (repos under the same directory) → suggest project from folder name
- **Duplicates/variants** (`myapp-v1`, `myapp-backup`) → flag for merge

A project is 1+ repos. The default is 1 repo per project. When multiple repos are detected as related, they are auto-merged into a single project. The user can always split a repo out or merge repos together later.

User confirms, renames, adjusts. Can assign repo roles (backend, frontend, library, docs, infra). Can skip grouping and do it later from the overview.

### Step 6: Done

Summary of what was configured. Redirect to the first project's dashboard, or to the overview if no projects were created.

Background: first index starts immediately for all discovered repos.

## What's Built

The setup wizard is fully implemented with all 6 steps. The ACP detection and configuration delegates to the daemon's `/api/acp/detect` and `/api/acp/configure` endpoints.

## What Needs Work

- **ACP step** should show the plugin install result for Claude Code (currently just says "MCP registered" even when the plugin installed successfully with commands/skills/hooks)
- **Group step** heuristics could be smarter — currently detects name stems, monorepos, and parent folders, but doesn't check GitHub org or cross-repo imports
- **Post-setup redirect** should go to the most relevant place (first project if one was created, overview otherwise)
