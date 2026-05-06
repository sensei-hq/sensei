# sensei daemon

Indexing daemon, CLI, and MCP server.

## Crates

| Crate | Purpose |
|-------|---------|
| `senseid` | Background daemon — HTTP API, indexing, inference |
| `sensei-cli` | Command-line interface |
| `sensei-mcp` | Model Context Protocol server |

## Database

DDL files in `database/` — the daemon owns schema migrations.

## Build

```bash
cargo build --release
```

## Run

```bash
senseid serve --port 9823
```
