#!/usr/bin/env bun
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { loadSenseiConfig } from "@sensei/shared";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();
const config = await loadSenseiConfig(repoPath);

if (!config) {
  // Exit cleanly — not inside a sensei-initialized repo.
  // Claude Code will show the server as unavailable (not as an error).
  process.exit(0);
}

// Register with collector daemon so OTLP events are attributed to this repo
fetch("http://localhost:51789/otlp/register", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ repoId: config.repo_id, repoPath }),
  signal: AbortSignal.timeout(500),
}).catch(() => { /* daemon not running — OTLP attribution silently skipped */ });

const server = createSenseiMcpServer({ repoId: config.repo_id, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
