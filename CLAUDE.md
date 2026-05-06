# sensei — monorepo

## Structure

| Directory   | Language     | Purpose                                |
|-------------|--------------|----------------------------------------|
| `app/`      | SvelteKit + Tauri | Desktop app (UI + native shell)   |
| `daemon/`   | Rust         | Backend API (senseid, sensei-cli, sensei-mcp) |
| `website/`  | SvelteKit    | Marketing website                      |
| `gateway/`  | Node.js      | LLM routing library                    |
| `docs/`     | Markdown     | Design docs, mockups, DDL, backlog     |

## Version

`VERSION` at the repo root is the single source of truth.
Run `make bump v=X.Y.Z` to update `VERSION`, `app/package.json`, and `daemon/Cargo.toml` atomically.

## Common commands

```bash
# Build daemon (dev)
make daemon-dev          # build only
make install-dev         # build + install to ~/.local/bin

# Build daemon (release)
make daemon-release
make install-release

# Run desktop app (dev)
make app-dev             # tauri dev with vite HMR

# Run all tests
make test

# Bump version across all manifests
make bump v=0.3.0
```

## Database

See `database/` for DDL. Dev builds connect to `sensei_dev`; release builds connect to `sensei`.
`DATABASE_URL` env var always overrides.

## Rules

- Always start with `docs/backlog.md`
- Create a todo list for complex tasks
- TDD — always use zero-errors-policy before starting work
- Commit and push when a logical chunk is complete
- Work in `develop` branch; merge to `main` when a feature is complete
- Marketplace (`sensei-hq/marketplace`) stays a separate repo but its version must match `VERSION`
