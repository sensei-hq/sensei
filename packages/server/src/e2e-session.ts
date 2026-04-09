#!/usr/bin/env bun
/**
 * E2E Scenario B: Session lifecycle
 * Tests: createSession → getSessionContext → takeSnapshot → checkpointTool
 * Verifies DB rows exist and session reaches 'completed' status.
 *
 * Usage (from repo root):
 *   bun packages/server/src/e2e-session.ts
 */

import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import {
  createSession,
  createTaskSession,
  takeSnapshot,
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

function pass(msg: string) {
  console.log(`  ✓ ${msg}`);
}
function fail(msg: string) {
  console.error(`  ✗ ${msg}`);
  process.exitCode = 1;
}

async function run() {
  console.log("=== E2E Scenario B: Session Lifecycle ===\n");

  // ── 1. Build client ──────────────────────────────────────────────────────
  console.log("1. Building Supabase client...");
  const client = await makeSenseiClient(REPO_PATH);
  if (!client) {
    fail("makeSenseiClient returned null — check credentials");
    return;
  }
  pass("client created");

  const config = await loadSenseiConfig(REPO_PATH);
  if (!config) {
    fail("loadSenseiConfig returned null");
    return;
  }
  const { repo_id: repoId } = config;
  pass(`repo_id: ${repoId}`);

  // ── 2. Create session + task_session ─────────────────────────────────────
  console.log("\n2. Creating session...");
  const session = await createSession(client as any, repoId);
  pass(`session created: ${session.id}`);

  const taskSession = await createTaskSession(
    client as any,
    session.id,
    repoId,
    "E2E Scenario B: session lifecycle test",
  );
  pass(`task_session created: ${taskSession.id}`);

  // ── 3. get_session_context ───────────────────────────────────────────────
  console.log("\n3. get_session_context...");
  const ctx = await getSessionContext(repoId, REPO_PATH, session.id);
  if (ctx.repo_name) pass(`repo_name: ${ctx.repo_name}`);
  else fail("missing repo_name");
  pass(`symbol_count: ${ctx.symbol_count}`);
  pass(`file_count: ${ctx.file_count}`);
  console.log(`     interrupted:    ${ctx.interrupted.length}`);

  // ── 4. take_snapshot ─────────────────────────────────────────────────────
  console.log("\n4. take_snapshot...");
  const snapshot = await takeSnapshot(client as any, session.id, repoId, {
    kind: "manual",
    progressSummary: "E2E test: running session lifecycle scenario",
    completedSteps: ["created session", "called get_session_context"],
    nextStepHint: "call checkpoint to close session",
  });
  if (snapshot.id) pass(`snapshot created: ${snapshot.id} (kind=${snapshot.kind})`);
  else fail("snapshot.id missing");

  const { data: dbSnap, error: snapErr } = await (client as any)
    .from("snapshots")
    .select("id,kind,progress_summary")
    .eq("id", snapshot.id)
    .single();
  if (snapErr || !dbSnap) fail(`snapshot not in DB: ${snapErr?.message}`);
  else pass(`snapshot verified in DB — progress_summary="${dbSnap.progress_summary}"`);

  // ── 5. checkpoint ────────────────────────────────────────────────────────
  console.log("\n5. checkpoint...");
  const cp = await checkpointTool(
    client as any,
    session.id,
    repoId,
    {
      task_summary: "E2E Scenario B completed — session lifecycle verified",
      completed_steps: [
        "created session",
        "called get_session_context",
        "took snapshot",
        "verified DB rows",
        "called checkpoint",
      ],
    },
    REPO_PATH,
  );
  if (cp.id) pass(`checkpoint snapshot: ${cp.id}`);
  else fail("checkpoint returned no snapshot id");

  await completeTaskSession(client as any, taskSession.id, session.id);
  pass("task_session completed");

  // ── 6. Verify session status ─────────────────────────────────────────────
  console.log("\n6. Verifying DB state...");
  const { data: dbSession, error: sessErr } = await (client as any)
    .from("sessions")
    .select("id,status")
    .eq("id", session.id)
    .single();
  if (sessErr || !dbSession) fail(`session not found: ${sessErr?.message}`);
  else if (dbSession.status === "completed") pass("session.status = completed");
  else fail(`session.status = ${dbSession.status} (expected completed)`);

  const { data: dbTs, error: tsErr } = await (client as any)
    .from("task_sessions")
    .select("id,status,ftr_score,task_type")
    .eq("id", taskSession.id)
    .single();
  if (tsErr || !dbTs) fail(`task_session not found: ${tsErr?.message}`);
  else {
    pass(`task_session.status = ${dbTs.status}`);
    pass(`task_session.task_type = ${dbTs.task_type}`);
    pass(`task_session.ftr_score = ${dbTs.ftr_score}`);
  }

  console.log("\n=== Done ===");
}

run().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
