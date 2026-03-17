# @sensei/mcp — DEPRECATED

> This package is superseded by the instrumented MCP server in `packages/server/src/mcp-entry.ts`.

## Why

This bare server (`packages/mcp/src/index.ts`) reads from local files and has no session tracking, no `beat()` heartbeat, no FTR scoring, and no Supabase integration. It was the original implementation before Supabase became the single source of truth.

## Use instead

The live MCP server is `packages/server/src/mcp-entry.ts`. It wraps every tool call with `beat()` for session tracking, reads from Supabase, and reports to the analytics dashboard.

`sensei init` and `sensei setup --mcp` both register the correct server in `~/.claude/mcp.json`.

This package is kept for reference only and should not be registered as an MCP server.
