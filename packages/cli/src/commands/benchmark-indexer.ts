import {
  reindexRepo,
  extractGroundTruth,
  loadSenseiIndex,
  connectCocoindex,
  score,
  printReport,
  writeMarkdownReport,
} from "@sensei/tools";
import { senseiPath } from "@sensei/shared";
import { existsSync } from "fs";

export async function benchmarkIndexer(repoPath: string): Promise<void> {
  const symbolMapPath = senseiPath(repoPath, "symbol-map.json");
  if (!existsSync(symbolMapPath)) {
    console.log("sensei: symbol-map.json not found, running indexer...");
    await reindexRepo(repoPath, { force: false });
  }

  console.log("Extracting ground truth from TypeScript exports...");
  const groundTruth = await extractGroundTruth(repoPath);
  console.log(`  Found ${groundTruth.files.length} TS source files, ${groundTruth.exportCount} exports`);

  console.log("Loading sensei index...");
  const senseiIndex = await loadSenseiIndex(repoPath);
  if (senseiIndex.missing) {
    console.error("sensei: symbol-map.json still missing after reindex. Something is wrong.");
    process.exit(1);
  }
  console.log(`  ${senseiIndex.symbols.length} symbols across ${senseiIndex.files.length} files`);

  console.log("Connecting to cocoindex-code MCP server (waiting for index)...");
  let cocoIndex;
  try {
    cocoIndex = await connectCocoindex(repoPath);
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`sensei: Failed to connect to cocoindex-code: ${msg}`);
    console.error("Make sure cocoindex-code is installed: pipx install cocoindex-code");
    console.error("And the index is built: cd <repo> && cocoindex-code index");
    process.exit(1);
  }
  console.log(`  ${cocoIndex.files.length} files indexed`);

  console.log("Scoring...");
  const report = await score(groundTruth, cocoIndex, senseiIndex);
  await cocoIndex.close();

  printReport(report);
  const outPath = await writeMarkdownReport(report, repoPath);
  console.log(`\nReport written to: ${outPath}`);
}
