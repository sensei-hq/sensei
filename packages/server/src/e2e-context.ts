#!/usr/bin/env bun
/**
 * E2E Scenario C: context_pack and recommend_next
 * Verifies that context_pack assembles slices and recommend_next returns relevant files.
 * Requires indexed repo (run e2e-index.ts first).
 *
 * Usage: bun packages/server/src/e2e-context.ts
 */

import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import { TransformersBackend } from "@sensei/engine";
import { contextPack } from "./tools/context-pack.js";
import { recommendNext } from "./tools/recommend-next.js";

const REPO_PATH = "/Users/Jerry/Developer/sensei";
const LOCAL_SERVICE_KEY =
  "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU";

if (!process.env.SUPABASE_SERVICE_KEY) {
  process.env.SUPABASE_SERVICE_KEY = LOCAL_SERVICE_KEY;
}

function pass(msg: string) { console.log(`  ✓ ${msg}`); }
function fail(msg: string) { console.error(`  ✗ ${msg}`); process.exitCode = 1; }

async function run() {
  console.log("=== E2E Scenario C: context_pack + recommend_next ===\n");

  const client = await makeSenseiClient(REPO_PATH);
  if (!client) { fail("No client"); return; }

  const config = await loadSenseiConfig(REPO_PATH);
  if (!config) { fail("No config"); return; }
  const { repo_id: repoId } = config;

  const backend = new TransformersBackend();

  // ── 1. recommend_next ────────────────────────────────────────────────────
  console.log("1. recommend_next: 'session lifecycle tracking'");
  const recs = await recommendNext(client as any, backend, repoId, "session lifecycle tracking");
  if (recs.recommendations && recs.recommendations.length > 0) {
    pass(`returned ${recs.recommendations.length} file recommendations`);
    for (const f of recs.recommendations.slice(0, 3)) {
      console.log(`     ${f.filePath} (~${f.estimatedTokens} tokens, score=${f.score?.toFixed(2)})`);
    }
    pass(`suggestedBudget: ${recs.suggestedBudget} tokens`);
  } else {
    fail("recommend_next returned no files");
  }

  // ── 2. context_pack ──────────────────────────────────────────────────────
  console.log("\n2. context_pack: 'FTR score computation'");
  const pack = await contextPack(
    client as any,
    backend,
    repoId,
    REPO_PATH,
    "FTR score computation",
    { maxTokens: 4000, modelId: "claude-sonnet-4-6" },
  );
  if (pack.id) pass(`pack id: ${pack.id}`);
  else fail("pack missing id");

  if (pack.slices && pack.slices.length > 0) {
    pass(`${pack.slices.length} slices assembled (${pack.totalTokens} tokens)`);
    console.log(`     top slice: ${pack.slices[0]?.filePath ?? "?"}`);
  } else {
    fail("pack has no slices");
  }

  if (pack.totalTokens > 0) pass(`totalTokens: ${pack.totalTokens}`);
  else fail("totalTokens = 0");

  // Verify context_packs row written to DB
  const { data: dbPack, error: packErr } = await (client as any)
    .from("context_packs")
    .select("id,total_tokens,task")
    .eq("id", pack.id)
    .single();
  if (packErr || !dbPack) fail(`context_pack not in DB: ${packErr?.message}`);
  else pass(`context_pack persisted — task="${dbPack.task}", tokens=${dbPack.total_tokens}`);

  console.log("\n=== Done ===");
}

run().catch(err => {
  console.error("Fatal:", err);
  process.exit(1);
});
