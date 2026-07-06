---
organization: Sensei HQ
project: sensei
role: monorepo
tagline: Development intelligence for AI-assisted coding
summary: Sensei watches coding sessions, learns your team's patterns and conventions, and feeds that knowledge back to the AI assistant so it gets things right the first time.
stack: [rust, sveltekit, svelte, tauri, typescript, postgres, ruby, markdown]
icon: sensei.svg
---

# Sensei

> Development intelligence for AI-assisted coding.

Sensei watches your coding sessions, learns your team's patterns and conventions, and feeds that knowledge back to your AI assistant — so it gets it right the first time.

## Repository structure

| Directory | Language | Purpose |
|-----------|----------|---------|
| [`app/`](app/) | SvelteKit + Tauri | Desktop observatory |
| [`crates/`](crates/) | Rust | Background daemon, CLI, MCP server (single unified workspace) |
| [`website/`](website/) | SvelteKit | Marketing website |
| [`docs/`](docs/) | Markdown | Design docs, mockups, backlog |

Sibling repos (versioned independently, pulled in as dependencies):

- **[sensei-hq/gateway](https://github.com/sensei-hq/gateway)** — LLM routing library. Consumed by the daemon as a git dependency (`gateway-embedded` in `crates/senseid/Cargo.toml`). Previously vendored here as `gateway/`; moved out so it can be released to crates.io on its own cadence.
- **[sensei-hq/marketplace](https://github.com/sensei-hq/marketplace)** — skills, commands, agents, and hooks. Version tracks this repo.

## Install (macOS)

```bash
brew tap sensei-hq/tap
brew install sensei-hq/tap/senseihq        # app + CLI + daemon
brew install --cask sensei-hq/tap/sensei # Desktop app
```

Homebrew formulae live in [sensei-hq/homebrew-tap](https://github.com/sensei-hq/homebrew-tap).

## Prerequisites (development)

- Rust stable + cargo
- Bun
- PostgreSQL (local) — dev uses `sensei_dev`, release uses `sensei`

## Quick start

```bash
# First-time setup: install git hooks (runs unit tests on each commit)
make setup-hooks

# Build daemon binaries (dev) and install to ~/.local/bin
make install-dev

# Run desktop app with hot reload
make app-dev

# Run marketing website
make website-dev

# Run all tests
make test
```

## Version bump

```bash
make bump v=0.3.0
# Updates: VERSION, app/package.json, daemon/crates/{senseid,cli,mcp}/Cargo.toml
```

## Dependency updates

```bash
make update
# cargo update + bun update (app + website) + make test
```

## Component READMEs

- [app/README.md](app/README.md) — desktop app setup, routes, build
- [website/README.md](website/README.md) — marketing site, deployment
- [docs/README.md](docs/README.md) — design docs index
- [sensei-hq/gateway](https://github.com/sensei-hq/gateway) — LLM routing, providers, capabilities (separate repo)
