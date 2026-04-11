#!/usr/bin/env bun
/**
 * senseid — Sensei background daemon.
 *
 * Starts the HTTP server, graph indexer, file watchers, and OTLP collector.
 * Designed to run as a persistent background service (launchd, systemd, brew services).
 *
 * Usage:
 *   senseid [--port <n>]   start the daemon
 *   senseid stop           send SIGTERM via PID file
 *   senseid status         check if daemon is running
 */
import { parseArgs } from "node:util";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { homedir } from "node:os";

const { positionals, values } = parseArgs({
  args: process.argv.slice(2),
  allowPositionals: true,
  options: {
    port: { type: "string" },
    help: { type: "boolean", short: "h", default: false },
  },
});

const [subCmd] = positionals;
const PID_FILE = join(homedir(), ".sensei", "serve.pid");

if (values.help || subCmd === "help") {
  console.log("senseid [--port <n>]   start daemon (default port 7744)");
  console.log("senseid stop           stop running daemon");
  console.log("senseid status         show daemon status");
  process.exit(0);
}

if (subCmd === "stop") {
  const port = values.port ? parseInt(values.port, 10) : 7744;
  try {
    const res = await fetch(`http://127.0.0.1:${port}/stop`, { method: "POST" });
    if (res.ok) {
      console.log("senseid: stopped.");
      process.exit(0);
    }
    // Server responded but endpoint missing (old binary) — fall through to PID fallback
  } catch {
    // Not responding on HTTP — fall through to PID fallback
  }
  // Fallback: kill via PID file
  try {
    const pid = parseInt((await readFile(PID_FILE, "utf-8")).trim(), 10);
    process.kill(pid, "SIGTERM");
    console.log(`senseid: sent SIGTERM to pid ${pid}`);
  } catch {
    console.error("senseid: not running");
    process.exit(1);
  }
  process.exit(0);
}

if (subCmd === "status") {
  const port = values.port ? parseInt(values.port, 10) : 7744;
  try {
    const res = await fetch(`http://127.0.0.1:${port}/health`);
    if (res.ok) {
      const health = await res.json() as Record<string, unknown>;
      console.log(`senseid: running on :${port}`);
      if (health.version) console.log(`  version: ${health.version}`);
      if (health.backend) console.log(`  backend: ${health.backend}`);
      const indexing = health.indexing as string[] | null;
      if (indexing?.length) console.log(`  indexing: ${indexing.join(", ")}`);
    } else {
      console.log("senseid: not running");
    }
  } catch {
    // Check PID file as fallback
    try {
      const pid = (await readFile(PID_FILE, "utf-8")).trim();
      console.log(`senseid: PID file exists (pid ${pid}) but port ${port} not responding`);
    } catch {
      console.log("senseid: not running");
    }
  }
  process.exit(0);
}

// Default: start the daemon
const { serve } = await import("@sensei/server");
await serve(process.cwd(), {
  port: values.port ? parseInt(values.port, 10) : undefined,
  daemon: true,
});
