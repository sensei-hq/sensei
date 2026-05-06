# sensei daemon

Backend daemon, CLI, and MCP server for Sensei.

## Crates

| Crate | Binary | Purpose |
|-------|--------|---------|
| `senseid` | `senseid` | HTTP API, indexing pipeline, task queue, file watcher, inference |
| `sensei-cli` | `sensei` | CLI — `init`, `serve`, `status`, `doctor` |
| `sensei-mcp` | `sensei-mcp` | MCP server — search, callers, patterns, sessions, libraries |

## Database

DDL files are in `database/`. The daemon owns schema migrations.

- Dev connects to `sensei_dev`
- Release connects to `sensei`
- `DATABASE_URL` env var always overrides

```bash
# Apply schema
dbd reset    # reset and apply DDL
dbd apply    # apply without reset
```

## Build

```bash
# From monorepo root (recommended)
make daemon-dev        # debug build
make daemon-release    # release build
make install-dev       # debug build + install to ~/.local/bin
make install-release   # release build + install to ~/.local/bin

# Or directly from this directory
cargo build -p senseid
cargo build -p sensei-cli
cargo build -p sensei-mcp
```

## Run

```bash
senseid serve --port 9823
sensei status
```

## Tests

```bash
# From monorepo root
make test-daemon

# Or directly
cargo test --workspace
```

## Coverage

```bash
# Requires cargo-llvm-cov (cargo install cargo-llvm-cov)
cargo coverage-all    # workspace summary (terminal)
cargo coverage-html   # HTML report → target/coverage/index.html
```
