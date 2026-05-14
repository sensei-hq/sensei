# Sensei Documentation

Observe. Learn. Improve.

## Structure

| Directory | Purpose |
|-----------|---------|
| `ideas/` | What Sensei does -- user perspective. Start here. |
| `design/` | How it's built -- engineering perspective. References ideas/. |
| `mockups/` | Visual reference -- HTML mockups and design system. |
| `archive/` | Old documentation -- retained for historical context. |
| `backlog.md` | Active implementation backlog. |

## Reading order

1. `ideas/README.md` -- product overview, module map, status
2. `ideas/01-bootstrap.md` through `ideas/06-logging.md` -- what the user experiences
3. `design/README.md` -- architecture, principles, component map
4. `design/01-app.md` through `design/10-build-and-release.md` -- how each component is built

## Monorepo structure

| Directory | Language | Purpose |
|-----------|----------|---------|
| `app/` | SvelteKit + Tauri | Desktop observatory |
| `crates/` | Rust | All Rust crates (senseid, cli, mcp, bootstrap, gateway) |
| `website/` | SvelteKit | Marketing site |
| `database/` | SQL | DDL definitions |
| `homebrew/` | Ruby | Homebrew tap (subtree) |
| `marketplace/` | Markdown | Skills and plugins (subtree) |
| `docs/` | Markdown | This documentation |
