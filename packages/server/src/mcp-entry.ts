#!/usr/bin/env bun
/**
 * MCP entry point for sensei.
 *
 * The MCP server runs as a stdio subprocess of Claude Code (or other ACPs).
 * It identifies the current repo by looking up the cwd in the central
 * project registry (~/.sensei/projects.json), then proxies tool calls
 * through the senseid daemon at localhost:7744.
 *
 * No direct Kuzu/SQLite access — the daemon is the single source of truth.
 */
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { lookupRepoId } from "@sensei/shared";
import { loadSenseiConfig } from "@sensei/shared";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();

// Resolve repoId: central registry first, fall back to .sensei/config.yaml
let repoId = await lookupRepoId(repoPath);

if (!repoId) {
  // Fallback: read from per-repo config (legacy or not yet registered)
  const config = await loadSenseiConfig(repoPath).catch(() => null);
  repoId = config?.repo_id;
}

if (!repoId) {
  // Not a sensei-managed repo — exit cleanly.
  // Claude Code will show the MCP server as unavailable.
  process.exit(0);
}

const server = createSenseiMcpServer({ repoId, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
