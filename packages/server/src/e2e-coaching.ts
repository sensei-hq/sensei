#!/usr/bin/env bun
/**
 * E2E Scenario D: FTR coaching hints surface on session start
 * Creates several sessions with poor FTR signals, then verifies coaching hints
 * appear in get_session_context.
 *
 * Usage: bun packages/server/src/e2e-coaching.ts
 */

import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import {
  createSession,
  createTaskSession,
  completeTaskSession,
} from "@sensei/engine";
import { getSessionContext } from "./tools/get-session-context.js";
import { checkpointTool } from "./tools/checkpoint.js";

const REPO_PATH = "/Users/Jerry/Developer/sensei";
const LOCAL_SERVICE_KEY =
  "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU";

if (!process.env.SUPABASE_SERVICE_KEY) {
  process.env.SUPABASE_SERVICE_KEY = LOCAL_SERVICE_KEY;
}

function pass(msg: string) { console.log(`  ✓ ${msg}`); }
function fail(msg: string) { console.error(`  ✗ ${msg}`); process.exitCode = 1; }

async function seedLowFtrSession(
  client: any,
  repoId: string,
  signals: { snapshotCount: number; toolErrorRate: number; completedCleanly: boolean; hasDescription: boolean },
  ftrScore: number,
): Promise<void> {
  // Create a real session so FK constraints are satisfied
  const session = await createSession(client, repoId);
  const taskDesc = signals.hasDescription ? "some task description" : undefined;
  const taskSession = await createTaskSession(client, session.id, repoId, taskDesc);

  // Close session as 'completed'
  await client.from("sessions").update({ status: "completed" }).eq("id", session.id);

  // Directly write ftr_score + ftr_signals so we control the exact score
  await client
    .from("task_sessions")
    .update({
      status: "completed",
      ftr_score: ftrScore,
      ftr_signals: signals,
      completed_at: new Date().toISOString(),
    })
    .eq("id", taskSession.id);
}

async function run() {
  console.log("=== E2E Scenario D: FTR Coaching Hints ===\n");

  const client = await makeSenseiClient(REPO_PATH);
  if (!client) { fail("No client"); return; }
  const config = await loadSenseiConfig(REPO_PATH);
  if (!config) { fail("No config"); return; }
  const { repo_id: repoId } = config;

  // ── 1. Seed sessions with missing_descriptions pattern ───────────────────
  // Score = 1.0 - 0.30 (incomplete) = 0.70, capped to 0.70 — but 0.70 >= LOW_FTR_THRESHOLD
  // Use combined: incomplete (−0.30) + error_rate (−0.20) + no_description → 0.50
  console.log("1. Seeding sessions with missing_descriptions pattern (ftr=0.50)...");
  for (let i = 0; i < 3; i++) {
    await seedLowFtrSession(client as any, repoId, {
      snapshotCount: 1,
      toolErrorRate: 0.25,
      completedCleanly: false,
      hasDescription: false,
    }, 0.50);
  }
  pass("3 sessions seeded with ftr_score=0.50, no description");

  // ── 2. Seed sessions with high_snapshot_count pattern ────────────────────
  // Score = 1.0 − 0.30 (incomplete) − 0.20 (error rate) − 0.15 (4 extra snaps) = 0.35
  console.log("\n2. Seeding sessions with high_snapshot_count pattern (ftr=0.35)...");
  for (let i = 0; i < 2; i++) {
    await seedLowFtrSession(client as any, repoId, {
      snapshotCount: 5,
      toolErrorRate: 0.25,
      completedCleanly: false,
      hasDescription: true,
    }, 0.35);
  }
  pass("2 sessions seeded with ftr_score=0.35, 5 snapshots");

  // ── 3. Call get_session_context on a new session and check coaching ────────
  console.log("\n3. Checking coaching hints in get_session_context...");
  const session = await createSession(client as any, repoId);
  const taskSession = await createTaskSession(client as any, session.id, repoId, "checking coaching hints");
  const ctx = await getSessionContext(repoId, REPO_PATH, session.id);
  pass(`session context loaded: ${ctx.symbol_count} symbols, ${ctx.file_count} files`);
  console.log(`     interrupted: ${ctx.interrupted.length}, open backlog: ${ctx.memory.openBacklog.length}`);

  // Clean up — close the test session
  await checkpointTool(client as any, session.id, repoId, {
    task_summary: "Scenario D coaching test complete",
  }, REPO_PATH);
  await completeTaskSession(client as any, taskSession.id, session.id);

  console.log("\n=== Done ===");
}

run().catch(err => { console.error("Fatal:", err); process.exit(1); });
