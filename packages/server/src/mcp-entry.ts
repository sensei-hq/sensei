#!/usr/bin/env bun
/**
 * MCP server entry point — launched by agent via stdio transport.
 * Usage: bun packages/server/src/mcp-entry.ts --repo-id <id> --repo-path <path>
 */
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { loadSenseiConfig } from "@sensei/shared";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();
const config = await loadSenseiConfig(repoPath);

if (!config) {
  console.error("[sensei-mcp] No .sensei/config.yaml found. Run sensei init first.");
  process.exit(1);
}

const server = createSenseiMcpServer({ repoId: config.repo_id, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
