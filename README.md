# Sensei

> Development intelligence for AI-assisted coding.

Sensei watches your coding sessions, learns your team's patterns and conventions, and feeds that knowledge back to your AI assistant — so it gets it right the first time.

## Repository structure

| Directory | Language | Purpose |
|-----------|----------|---------|
| [`app/`](app/) | SvelteKit + Tauri | Desktop observatory |
| [`daemon/`](daemon/) | Rust | Background daemon, CLI, MCP server |
| [`website/`](website/) | SvelteKit | Marketing website |
| [`gateway/`](gateway/) | Rust | LLM routing library |
| [`docs/`](docs/) | Markdown | Design docs, mockups, backlog |

**[marketplace](https://github.com/sensei-hq/marketplace)** — skills, commands, agents, and hooks — is a separate repo whose version tracks this one.

## Install (macOS)

```bash
brew tap sensei-hq/tap
brew install sensei        # CLI + daemon
brew install --cask sensei # Desktop app
```

Homebrew formulae live in [sensei-hq/homebrew-tap](https://github.com/sensei-hq/homebrew-tap).

## Prerequisites (development)

- Rust stable + cargo
- Bun
- PostgreSQL (local) — dev uses `sensei_dev`, release uses `sensei`

## Quick start

```bash
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
# cargo update (daemon + gateway) + bun update (app + website) + make test
```

## Component READMEs

- [app/README.md](app/README.md) — desktop app setup, routes, build
- [daemon/README.md](daemon/README.md) — crates, database, build targets
- [website/README.md](website/README.md) — marketing site, deployment
- [gateway/README.md](gateway/README.md) — LLM routing, providers, capabilities
- [docs/README.md](docs/README.md) — design docs index
