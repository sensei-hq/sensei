# sensei docs  先生

Observe. Learn. Improve.

Sensei is a development intelligence platform for AI-assisted coding. It watches your coding sessions, learns your team's patterns and conventions, and feeds that knowledge back to your AI assistant — so it gets it right the first time.

## This directory

Design docs, ideas, mockups, journeys, and analysis. No runnable code.

| Directory | Contents |
|-----------|----------|
| `ideas/` | Feature explorations and proposals |
| `mockups/` | Interactive JSX prototypes and design summaries |
| `journeys/` | User journeys and system pipelines |
| `design/` | Architecture, schemas, algorithms, ADRs |
| `blueprints/` | Implementation blueprints |
| `analysis/` | Gap analysis, skill-command mappings |
| `plans/` | Implementation plans |
| `features/` | Feature specifications |
| `experiments/` | Design experiments |
| `feature-requests/` | Upstream requests (Claude Code, Copilot, etc.) |
| `reference/` | Reference materials and blocking gaps |
| `backlog.md` | Active backlog — start here |

## Monorepo layout

All code lives in [sensei-hq/sensei](https://github.com/sensei-hq/sensei):

| Path | What |
|------|------|
| [`app/`](../app/) | Tauri + SvelteKit desktop app |
| [`daemon/`](../daemon/) | Rust workspace — senseid, sensei-cli, sensei-mcp, database DDL |
| [`website/`](../website/) | Marketing website |
| [`gateway/`](../gateway/) | LLM routing library |

## Separate repos

| Repo | Purpose |
|------|---------|
| [marketplace](https://github.com/sensei-hq/marketplace) | Skills, commands, agents, hooks, templates |
| [homebrew-tap](https://github.com/sensei-hq/homebrew-tap) | Homebrew formula and cask for macOS |
| [corpus](https://github.com/sensei-hq/corpus) | Benchmark repositories for indexing tests |
