# DEPRECATED

This TypeScript HTTP server (senseid daemon) is superseded by the Rust daemon at `crates/senseid/`.

The Rust implementation provides the same HTTP API surface with:
- No file watcher connection exhaustion (lazy DB per batch)
- Single binary distribution (no bun runtime required)
- Tree-sitter AST parsing (no regex adapters)
- Single SQLite DB for all repos (no per-repo DB files)

The MCP server (mcp-server.ts, mcp-entry.ts) remains active — it proxies
through the daemon's HTTP API and is ACP-independent.

This package is retained for the MCP server and reference during migration.
Do not add new features to the daemon/indexer portions.
