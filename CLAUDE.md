# sensei — monorepo

## Structure

| Directory     | Language          | Purpose                                       |
|---------------|-------------------|-----------------------------------------------|
| `app/`        | SvelteKit + Tauri | Desktop app (UI + native shell)               |
| `crates/`     | Rust              | All Rust crates — single unified workspace    |
| ├ `senseid`   |                   | HTTP daemon (API server)                      |
| ├ `cli`       |                   | sensei CLI (binary: sensei)                   |
| ├ `mcp`       |                   | MCP server                                    |
| ├ `bootstrap` |                   | Installer / prereq checker                    |
| └ `gateway`   |                   | LLM routing library                           |
| `website/`    | SvelteKit         | Marketing website                             |
| `docs/`       | Markdown          | Design docs, mockups, DDL, backlog            |
| `homebrew/`   | Ruby              | Homebrew tap (subtree → sensei-hq/homebrew-tap) |
| `marketplace/`| JSON/Markdown     | Skills & plugins (subtree → sensei-hq/marketplace) |

## Version

`VERSION` at the repo root is the single source of truth.
Run `make bump v=X.Y.Z` to update all manifests, commit, tag, push, and sync subtrees.

## Common commands

```bash
# Build Rust crates (dev)
make crates-dev          # build senseid + sensei-cli + sensei-mcp
make install-dev         # build + install to ~/.local/bin

# Build Rust crates (release)
make crates-release
make install-release

# Run desktop app (dev)
make app-dev             # tauri dev with vite HMR

# Run all tests
make test
make test-fast           # no DB required (pre-commit)

# Bump version across all manifests + tag + push
make bump v=0.3.0
```

## Database

See `database/` for DDL. Dev builds connect to `sensei_dev`; release builds connect to `sensei`.
`DATABASE_URL` env var always overrides (set in `.env.dev` for local dev).

## Rules

- Always start with `docs/backlog.md`
- Create a todo list for complex tasks
- TDD — always use zero-errors-policy before starting work
- Commit and push when a logical chunk is complete
- Work in `develop` branch; merge to `main` when a feature is complete
- `homebrew/` and `marketplace/` are git subtrees — edit in-repo, sync with `make tap-push` / `make marketplace-push`
