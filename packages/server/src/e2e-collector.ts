#!/usr/bin/env bun
/**
 * E2E Scenario E: collector drains JSONL events into local Supabase.
 * Tests the full pipeline: JSONL file → drainJsonl() → sensei.events table.
 *
 * Usage: bun packages/server/src/e2e-collector.ts
 */

import { writeFile, unlink } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import { drainJsonl } from "../../collector/src/drain.js";

const REPO_PATH = "/Users/Jerry/Developer/sensei";
const LOCAL_SERVICE_KEY =
  "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU";

if (!process.env.SUPABASE_SERVICE_KEY) {
  process.env.SUPABASE_SERVICE_KEY = LOCAL_SERVICE_KEY;
}

// Stable test UUID — won't exist in auth.users but events.user_uuid has no FK
const TEST_USER_UUID = "00000000-test-0000-0000-000000000001";

function pass(msg: string) { console.log(`  ✓ ${msg}`); }
function fail(msg: string) { console.error(`  ✗ ${msg}`); process.exitCode = 1; }

async function run() {
  console.log("=== E2E Scenario E: Collector JSONL Drain ===\n");

  const client = await makeSenseiClient(REPO_PATH);
  if (!client) { fail("No client"); return; }
  const config = await loadSenseiConfig(REPO_PATH);
  if (!config) { fail("No config"); return; }

  // ── 1. Write a temp JSONL file with 3 tool events ────────────────────────
  console.log("1. Writing JSONL event file...");
  const jsonlPath = join(tmpdir(), `sensei-e2e-${Date.now()}.jsonl`);
  const now = Date.now();
  const events = [
    { user_uuid: TEST_USER_UUID, phase: "pre",  tool: "Bash",   ts: now,       project_path: REPO_PATH, seq: 1, duration_ms: null, success: null },
    { user_uuid: TEST_USER_UUID, phase: "post", tool: "Bash",   ts: now + 100, project_path: REPO_PATH, seq: 1, duration_ms: 100,  success: true  },
    { user_uuid: TEST_USER_UUID, phase: "pre",  tool: "Edit",   ts: now + 200, project_path: REPO_PATH, seq: 2, duration_ms: null, success: null },
    { user_uuid: TEST_USER_UUID, phase: "post", tool: "Edit",   ts: now + 300, project_path: REPO_PATH, seq: 2, duration_ms:  50,  success: true  },
    { user_uuid: TEST_USER_UUID, phase: "pre",  tool: "Write",  ts: now + 400, project_path: REPO_PATH, seq: 3, duration_ms: null, success: null  },
    { user_uuid: TEST_USER_UUID, phase: "post", tool: "Write",  ts: now + 500, project_path: REPO_PATH, seq: 3, duration_ms:  80,  success: false, error: "disk full" },
  ];
  await writeFile(jsonlPath, events.map(e => JSON.stringify(e)).join("\n") + "\n");
  pass(`wrote ${events.length} events to ${jsonlPath}`);

  // ── 2. Drain the JSONL into Supabase ─────────────────────────────────────
  console.log("\n2. Draining JSONL into local Supabase...");
  await drainJsonl(client as any, jsonlPath);
  pass("drainJsonl completed");

  // Verify file was deleted (drain success)
  const { existsSync } = await import("fs");
  if (!existsSync(jsonlPath)) pass("JSONL file deleted after successful drain");
  else { fail("JSONL file still exists — drain may have failed"); await unlink(jsonlPath).catch(() => {}); }

  // ── 3. Verify events in DB ───────────────────────────────────────────────
  console.log("\n3. Verifying events in sensei.events...");
  const tsLow  = new Date(now - 1000).toISOString();
  const tsHigh = new Date(now + 2000).toISOString();
  const { data: rows, error } = await (client as any)
    .from("events")
    .select("tool,phase,success,duration_ms,user_uuid")
    .eq("user_uuid", TEST_USER_UUID)
    .gte("ts", tsLow)
    .lte("ts", tsHigh)
    .order("ts");

  if (error) { fail(`events query failed: ${error.message}`); return; }
  if (!rows || rows.length === 0) { fail("No events found in DB"); return; }

  pass(`${rows.length} events found in DB`);

  const postEvents = rows.filter((r: any) => r.phase === "post");
  const failedPost = rows.find((r: any) => r.phase === "post" && r.success === false);

  if (postEvents.length === 3) pass("3 post-phase events recorded");
  else fail(`expected 3 post-phase events, got ${postEvents.length}`);

  if (failedPost) pass(`failed event recorded: tool=${failedPost.tool}, success=false`);
  else fail("failed tool event not found");

  const avgDuration = postEvents.reduce((s: number, r: any) => s + (r.duration_ms ?? 0), 0) / postEvents.length;
  pass(`average post-event duration: ${avgDuration}ms`);

  console.log("\n=== Done ===");
}

run().catch(err => { console.error("Fatal:", err); process.exit(1); });
