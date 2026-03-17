#!/usr/bin/env bun
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { loadSenseiConfig, makeSenseiClient } from "@sensei/shared";
import { createOtlpEndpoint } from "./otlp-endpoint.js";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();
const config = await loadSenseiConfig(repoPath);

if (!config) {
  console.error("[sensei-mcp] No .sensei/config.yaml found. Run sensei init first.");
  process.exit(1);
}

// Start OTLP endpoint for Claude Code telemetry
const otlpPort = parseInt(process.env.SENSEI_OTEL_PORT ?? "4318", 10);
const dryRun = process.env.SENSEI_OTEL_DRY_RUN === "true";
let supabaseClient: any = null;
try { supabaseClient = await makeSenseiClient(repoPath); } catch { /* no client — write mode silently skipped */ }

const otlp = createOtlpEndpoint({ port: otlpPort, dryRun, repoId: config.repo_id, supabaseClient });
console.error(`[sensei-otel] Listening on :${otlp.port} (${dryRun ? "dry-run" : "live"})`);

const server = createSenseiMcpServer({ repoId: config.repo_id, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
