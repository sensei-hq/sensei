---
organization: Sensei HQ
workspace: sensei
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
| [`app/`](app/) | SvelteKit + Tauri | Desktop app (the observatory and workspace windows) |
| [`crates/`](crates/) | Rust | One workspace: `senseid` (daemon), `cli` (`sensei`), `mcp`, `bootstrap`, `logger`, `dojo-protocol`, `transcript-formats` |
| [`dojo/`](dojo/) | SvelteKit | The dōjō, the team web app (kavach + Supabase) |
| [`packages/`](packages/) | TypeScript + Svelte | Code shared by the app and the dōjō: `state` (domain state, the Relay first), `ui` (one screen layout), `codemap`, `glyphs`, `sdlc-figures` |
| [`database/`](database/) | SQL ([dbd](#prerequisites-development)) | The daemon's schema: DDL under `ddl/`, `design.yaml`, imports, policies, SQL tests. The files are the source of truth |
| [`supabase/`](supabase/) | SQL | The dōjō's local Supabase stack |
| [`docs/`](docs/) | Markdown | Product, roadmap, architecture, design, decisions, backlog — start at [docs/README.md](docs/README.md) |
| [`openspec/`](openspec/) | Markdown | Spec-first change proposals ([OpenSpec](https://github.com/Fission-AI/OpenSpec)): `openspec/changes/<id>/` |
| [`scripts/`](scripts/) | Shell, Python | Guards the hooks run, release and install helpers |
| [`homebrew/`](homebrew/) | Ruby | Formula and cask templates, rendered by the release into [sensei-hq/homebrew-tap](https://github.com/sensei-hq/homebrew-tap) |
| [`marketplace/`](marketplace/) | JSON, Markdown | Skills, commands, agents and hooks, mirrored to [sensei-hq/marketplace](https://github.com/sensei-hq/marketplace) on release |
| [`tools/`](tools/) | Rust | Stand-alone tools (`session-report`) |

Sibling repositories (released on their own schedule):

- **[sensei-hq/gateway](https://github.com/sensei-hq/gateway)** — LLM routing library, consumed by the daemon as a git dependency (`gateway-embedded` in `crates/senseid/Cargo.toml`).
- **[sensei-hq/website](https://github.com/sensei-hq/website)** — sensei-hq.com, checked out beside this repository.

## Install (macOS)

```bash
brew tap sensei-hq/tap
brew trust sensei-hq/tap                   # Homebrew 7+ loads a third-party tap only once trusted
brew install sensei-hq/tap/sensei          # CLI + daemon + MCP server
brew install --cask sensei-hq/tap/senseihq # Desktop app
```

Homebrew formulae live in [sensei-hq/homebrew-tap](https://github.com/sensei-hq/homebrew-tap).

## Install (Linux)

The desktop app ships as a `.deb` (Ubuntu 22.04 or newer, x86_64) on each
[release](https://github.com/sensei-hq/sensei/releases). Its first-run health check installs
the rest through Homebrew — PostgreSQL 17 with pgvector, Ollama and the sensei binaries — then
creates the database and starts the daemon. It needs Homebrew itself first:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
echo 'eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"' >> ~/.bashrc && source ~/.bashrc
brew tap sensei-hq/tap && brew trust sensei-hq/tap
sudo apt install ./Sensei_<version>_amd64.deb   # then open Sensei from the app menu
```

Without the desktop app: `brew install sensei-hq/tap/sensei pgvector`, then `sensei start`.

### Indexing engines

The indexer places call edges with each language's own compiler:

- **Rust** — rust-analyzer plus the standard library's sources
  (`rustup component add rust-analyzer rust-src`).
- **TypeScript / JavaScript / Svelte** — `node`, loading `typescript`,
  `svelte2tsx`, `svelte` and `@jridgewell/trace-mapping` from the repository, or
  else from `~/.sensei/engines` (versions pinned in
  `crates/senseid/ts-driver/package.json`).

`sensei doctor` lists them under **Indexing engines** and exits non-zero when
one is missing; `sensei doctor --fix` installs the missing ones. The daemon
also installs missing engines in the background at startup. Neither installs
rustup or Node.js itself — install those first.

### Watchdog

A background task that runs past its time limit is abandoned by the watchdog. It
stops; it does not fail loudly. `sensei doctor` shows the last 7 days of these
under **Watchdog**: a count per day and task kind, and the tasks abandoned most
often, by path. They are a warning. The doctor exits non-zero only when the
daemon cannot answer. The same figures are at `GET /api/tasks/abandoned?days=N`.

## Prerequisites (development)

- macOS (the desktop app and the Keychain), Xcode command line tools
- Rust stable via rustup, with `rust-analyzer` and `rust-src` (the indexer uses them)
- [Bun](https://bun.sh) — the JavaScript package manager and runner for `app/`, `dojo/` and `packages/`
- Node.js — the TypeScript indexing engine runs on it
- PostgreSQL 17, local on :5432 — the daemon's `sensei` database; tests use `sensei_test` (`make test-template`) and the e2e suite `sensei_e2e`
- [dbd](https://github.com/sensei-hq/dbd) 0.27 (`cargo install dbd-cli`) — applies and reconciles `database/`
- Supabase CLI and Docker — only for the local dōjō (`make dojo-local`)
- [OpenSpec](https://github.com/Fission-AI/OpenSpec) 0.17 (`bun add -g @fission-ai/openspec@0.17.2`) — to validate change proposals (`openspec validate <id> --strict`)

## Quick start

```bash
make setup-hooks        # once: the pre-commit hook runs the fast suite and the guards
make crates             # build senseid, sensei and sensei-mcp (release)
make install            # install them over the brew install, and the desktop app
make app-dev            # the desktop app with hot reload
make dojo-local         # the local dōjō: Supabase + dojo dev on 127.0.0.1:5173
make test-fast          # no database needed (what the hook runs)
make test               # everything, database tests included
```

`make install-service` installs the daemon alone; each reinstall asks the Keychain for "Always Allow" again (#283).
E2E: `make test-app-e2e` (the installed daemon on a throwaway `sensei_e2e`).

## Version and release

A release ships from `main` by version tag. Every release bumps the version; `VERSION` is the single source.

```bash
make bump v=patch       # 0.22.1 → 0.22.2   (on a clean develop; commits and pushes develop)
make bump v=minor       # 0.22.1 → 0.23.0
make bump v=major       # 0.22.1 → 1.0.0
make bump v=0.30.0      # an explicit version
# then: PR develop → main, merged when green; on main:
make release            # tags main, pushes the tag, syncs the marketplace, merges main back into develop
```

The tag runs `release.yml`: the builds, the GitHub release, the Homebrew tap and the dōjō database (approved by a person).
`scripts/release-guard.sh` refuses either step on the wrong branch. Details: [CLAUDE.md](CLAUDE.md#version).

## Dependency updates

```bash
make update             # cargo update + bun update (app), then make test
```

## Documentation

- [docs/README.md](docs/README.md) — the index: product (`prd/`), status (`roadmap/`), architecture, design, journeys
- [docs/backlog.md](docs/backlog.md) — open work, as GitHub issue references
- [docs/decisions.md](docs/decisions.md) — the decision log
- [openspec/project.md](openspec/project.md) — the project context for spec-first changes; [AGENTS.md](AGENTS.md) for assistants
- [app/README.md](app/README.md) — the desktop app: setup, routes, build
- [sensei-hq/gateway](https://github.com/sensei-hq/gateway) — LLM routing, providers, capabilities
