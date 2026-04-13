#!/usr/bin/env bun
/**
 * senseid — Sensei background daemon.
 *
 * Usage:
 *   senseid start [--port <n>]  Start as background daemon
 *   senseid stop                Stop the running daemon
 *   senseid status              Check if daemon is running
 *   senseid logs                Tail the daemon log
 *   senseid [--port <n>]        Start in foreground (for dev)
 */
import { parseArgs } from "node:util";
import { readFile, writeFile, unlink } from "node:fs/promises";
import { join } from "node:path";
import { homedir } from "node:os";
import { existsSync } from "node:fs";
import { spawn } from "node:child_process";

const { positionals, values } = parseArgs({
  args: process.argv.slice(2),
  allowPositionals: true,
  options: {
    port: { type: "string" },
    help: { type: "boolean", short: "h", default: false },
  },
});

const [subCmd] = positionals;
const SENSEI_DIR = join(homedir(), ".sensei");
const PID_FILE = join(SENSEI_DIR, "serve.pid");
const LOG_FILE = join(SENSEI_DIR, "senseid.log");

function getPort(): number {
  return values.port ? parseInt(values.port, 10) : 7744;
}

if (values.help || subCmd === "help") {
  console.log(`senseid — Sensei background daemon

Commands:
  senseid start [--port <n>]  Start daemon in background (default port 7744)
  senseid stop                Stop the running daemon
  senseid status              Show daemon status
  senseid logs                Tail the daemon log file
  senseid [--port <n>]        Start in foreground (for development)`);
  process.exit(0);
}

// ── STOP ──────────────────────────────────────────────────────────────────────

if (subCmd === "stop") {
  const port = getPort();

  // Try HTTP stop first
  try {
    const res = await fetch(`http://127.0.0.1:${port}/stop`, { method: "POST", signal: AbortSignal.timeout(3000) });
    if (res.ok) {
      console.log("senseid: stopped.");
      // Clean PID file
      await unlink(PID_FILE).catch(() => {});
      process.exit(0);
    }
  } catch { /* not responding */ }

  // Fallback: kill via PID file
  try {
    const pidStr = await readFile(PID_FILE, "utf-8");
    const pid = parseInt(pidStr.trim(), 10);
    if (pid > 0) {
      try {
        process.kill(pid, "SIGTERM");
        console.log(`senseid: sent SIGTERM to pid ${pid}`);
        await unlink(PID_FILE).catch(() => {});
        process.exit(0);
      } catch (e: any) {
        if (e.code === "ESRCH") {
          // Process doesn't exist — clean stale PID
          await unlink(PID_FILE).catch(() => {});
          console.log("senseid: not running (cleaned stale PID file)");
          process.exit(0);
        }
        throw e;
      }
    }
  } catch { /* no PID file */ }

  console.error("senseid: not running");
  process.exit(1);
}

// ── STATUS ────────────────────────────────────────────────────────────────────

if (subCmd === "status") {
  const port = getPort();
  try {
    const res = await fetch(`http://127.0.0.1:${port}/health`, { signal: AbortSignal.timeout(3000) });
    if (res.ok) {
      const health = await res.json() as Record<string, unknown>;
      console.log(`senseid: running on :${port}`);
      if (health.version) console.log(`  version: ${health.version}`);
      if (health.backend) console.log(`  backend: ${health.backend}`);
      const indexing = health.indexing as string[] | null;
      if (indexing?.length) console.log(`  indexing: ${indexing.join(", ")}`);

      // Show PID
      try {
        const pid = (await readFile(PID_FILE, "utf-8")).trim();
        console.log(`  pid: ${pid}`);
      } catch { /* no PID file */ }
    } else {
      console.log("senseid: not running (health check failed)");
    }
  } catch {
    // Check PID file
    try {
      const pid = (await readFile(PID_FILE, "utf-8")).trim();
      // Check if process exists
      try {
        process.kill(parseInt(pid, 10), 0); // signal 0 = existence check
        console.log(`senseid: PID ${pid} exists but port ${port} not responding`);
      } catch {
        await unlink(PID_FILE).catch(() => {});
        console.log("senseid: not running (cleaned stale PID file)");
      }
    } catch {
      console.log("senseid: not running");
    }
  }
  process.exit(0);
}

// ── LOGS ──────────────────────────────────────────────────────────────────────

if (subCmd === "logs") {
  if (!existsSync(LOG_FILE)) {
    console.error(`senseid: no log file at ${LOG_FILE}`);
    process.exit(1);
  }
  // Tail the log file
  const proc = spawn("tail", ["-f", "-n", "50", LOG_FILE], { stdio: "inherit" });
  proc.on("exit", (code) => process.exit(code ?? 0));
  // Forward Ctrl+C
  process.on("SIGINT", () => { proc.kill("SIGINT"); process.exit(0); });
  // Block exit
  await new Promise(() => {});
}

// ── START (background) ────────────────────────────────────────────────────────

if (subCmd === "start") {
  const port = getPort();

  // Check if already running
  try {
    const res = await fetch(`http://127.0.0.1:${port}/health`, { signal: AbortSignal.timeout(2000) });
    if (res.ok) {
      console.log(`senseid: already running on :${port}`);
      process.exit(0);
    }
  } catch { /* not running — good */ }

  // Find our own binary path
  const binPath = process.argv[1];

  // Spawn in background (no subCmd = foreground mode)
  const out = Bun.file(LOG_FILE).writer();
  const child = spawn("bun", [binPath, "--port", String(port)], {
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env },
  });

  // Write PID
  if (child.pid) {
    await writeFile(PID_FILE, String(child.pid));
  }

  // Pipe output to log file
  child.stdout?.on("data", (d: Buffer) => out.write(d));
  child.stderr?.on("data", (d: Buffer) => out.write(d));

  child.unref();

  // Wait a moment for startup
  await new Promise(r => setTimeout(r, 2000));

  // Check if it started
  try {
    const res = await fetch(`http://127.0.0.1:${port}/health`, { signal: AbortSignal.timeout(3000) });
    if (res.ok) {
      console.log(`senseid: started on :${port} (pid ${child.pid})`);
      console.log(`  logs: ${LOG_FILE}`);
    } else {
      console.error("senseid: started but health check failed");
    }
  } catch {
    console.error("senseid: failed to start — check logs:");
    console.error(`  ${LOG_FILE}`);
  }

  process.exit(0);
}

// ── FOREGROUND (default, no subCmd) ───────────────────────────────────────────

// Clean PID on exit
function cleanPid() {
  try { require("fs").unlinkSync(PID_FILE); } catch { /* ignore */ }
}
process.on("SIGINT", () => { cleanPid(); process.exit(0); });
process.on("SIGTERM", () => { cleanPid(); process.exit(0); });
process.on("exit", cleanPid);

const { serve } = await import("@sensei/server");
await serve(process.cwd(), {
  port: values.port ? parseInt(values.port, 10) : undefined,
  daemon: true,
});
