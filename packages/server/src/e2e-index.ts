#!/usr/bin/env bun
/**
 * E2E Scenario A helper: run the engine indexer to populate symbols + scan_state.
 * Usage: bun packages/server/src/e2e-index.ts
 */

import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import { indexRepo } from "@sensei/engine";

const REPO_PATH = "/Users/Jerry/Developer/sensei";
const LOCAL_SERVICE_KEY =
  "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU";

if (!process.env.SUPABASE_SERVICE_KEY) {
  process.env.SUPABASE_SERVICE_KEY = LOCAL_SERVICE_KEY;
}

async function run() {
  console.log("=== E2E: Engine Indexer ===\n");

  const client = await makeSenseiClient(REPO_PATH);
  if (!client) { console.error("✗ No client"); process.exit(1); }

  const config = await loadSenseiConfig(REPO_PATH);
  if (!config) { console.error("✗ No config"); process.exit(1); }

  console.log(`Indexing repo_id: ${config.repo_id}`);
  const result = await indexRepo({ repoPath: REPO_PATH, repoId: config.repo_id, client: client as any });

  console.log(`\n  symbolsUpserted: ${result.symbolsUpserted}`);
  console.log(`  filesIndexed:    ${result.filesIndexed}`);
  console.log(`  filesDeleted:    ${result.filesDeleted}`);
  console.log(`  errors:          ${result.errors?.join(", ") || "none"}`);
  console.log("\nDone.");
}

run().catch(err => { console.error("Fatal:", err); process.exit(1); });
