# sensei docs  先生

Private design documentation for sensei — an AI development intelligence platform that observes coding sessions, detects patterns, and teaches assistants to get it right the first time.

## This repo

Design docs, ideas, mockups, journeys, and analysis. No runnable code.

| Directory | Contents |
|-----------|----------|
| `ideas/` | Feature explorations and proposals (31 numbered ideas) |
| `mockups/` | Interactive JSX prototypes, design summary, gap analysis |
| `journeys/` | User journeys (J1-J9) and system pipelines |
| `design/` | Architecture, schemas, algorithms, ADRs |
| `blueprints/` | Implementation blueprints |
| `analysis/` | Gap analysis, skill-command mappings |
| `plans/` | Implementation plans |
| `features/` | Feature specifications |
| `experiments/` | Design experiments |
| `feature-requests/` | Upstream feature requests (Claude Code, Copilot, etc.) |
| `reference/` | Reference materials, blocking gaps |

## Workspace repos

### [daemon](https://github.com/sensei-hq/daemon) — core engine

Rust workspace with three crates and the database schema.

| Path | What |
|------|------|
| `crates/senseid/` | Background daemon — HTTP API, indexing pipeline, task queue, file watcher, inference |
| `crates/sensei-cli/` | CLI (`sensei init`, `sensei serve`, `sensei status`, `sensei doctor`) |
| `crates/sensei-mcp/` | MCP server — search, callers, patterns, sessions, libraries |
| `database/` | PostgreSQL DDL — enums, tables, views, functions, procedures, seed data, RLS policies |

### [app](https://github.com/sensei-hq/app) — desktop observatory

Tauri + SvelteKit + Svelte 5 desktop app. Three artboards: bootstrap, setup wizard, observatory.

| Path | What |
|------|------|
| `src/routes/(app)/` | Observatory pages — today, sessions, learnings, libraries, instruments, settings, projects/[id] |
| `src/routes/(config)/` | Setup wizard — 11 stages from welcome through assignments |
| `src/routes/(health)/` | Daemon health check gate |
| `src/lib/` | API client, app state, design tokens, shared components, daemon lifecycle |
| `src-tauri/` | Tauri Rust backend — daemon check/start commands |

### [gateway](https://github.com/sensei-hq/gateway) — LLM router

Rust library for multi-provider LLM routing. Fallback chains, circuit breakers, cost tracking.

| What | Details |
|------|---------|
| Providers | Ollama (local), Anthropic, OpenAI, Google |
| Capabilities | chat, reasoning, embed, classify, summarize, vision, audio |
| Features | Fallback chains per capability, per-request cost tracking, circuit breaker per endpoint |

### [marketplace](https://github.com/sensei-hq/marketplace) — extensions

Skills, commands, agents, hooks, templates, and mindsets that ship with sensei.

| Path | What |
|------|------|
| `plugins/sensei/skills/` | Skills — codebase indexing, test gen, refactor, extract docs, reverse engineering |
| `plugins/sensei/commands/` | Slash commands — brainstorm, analyze, blueprint, plan, build, validate, review |
| `plugins/sensei/agents/` | Mindset agents — analyst, developer, acceptance-tester, security, UX, performance, DevOps |
| `plugins/sensei/hooks/` | Session lifecycle hooks — session-start, pre-tool, post-tool, pre-compact, user-prompt |
| `templates/` | Phase document templates — idea, analysis, blueprint, experiment, plan |
| `mindsets/` | Mindset definitions (7 mindsets, 3 core + 4 specialist) |

### [homebrew-tap](https://github.com/sensei-hq/homebrew-tap) — distribution

Homebrew formula and cask for macOS installation.

| File | What |
|------|------|
| `Formula/sensei.rb` | Installs sensei-cli, senseid daemon, sensei-mcp from release binaries |
| `Casks/sensei-app.rb` | Installs the desktop app |

### [website](https://github.com/sensei-hq/website) — website

Marketing website built with SvelteKit + Rokkit UI.

### [corpus](https://github.com/sensei-hq/corpus) — benchmarks

Benchmark repositories for testing sensei's indexing and coaching.

| Repo | Stack | Purpose |
|------|-------|---------|
| `repos/sample/` | TypeScript, Hono, Zod | REST API with routes, services, tests |
| `repos/httpx/` | Python | HTTP client library |
| `repos/multi-lang/` | Java, Kotlin, Swift, Python, Rust, SQL | Polyglot indexing test |
| `repos/task-manager/` | TypeScript | Minimal API with benchmark prompts |

### [releases](https://github.com/sensei-hq/releases) — binaries

Pre-built release binaries for download. Updated by CI on daemon tags.
