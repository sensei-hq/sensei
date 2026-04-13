# DEPRECATED

This TypeScript graph indexer is superseded by the Rust daemon at `crates/senseid/`.

The Rust implementation:
- Uses tree-sitter native (no Node.js native module issues)
- Single SQLite graph DB (no Kuzu connection exhaustion)
- 17MB single binary (no bun, no node_modules)
- Indexes the sensei monorepo in 803ms
- 94 tests

This package is retained for reference during the migration period.
Do not add new features here.
