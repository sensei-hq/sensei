import { intro, outro, note, log } from "@clack/prompts";
import { existsSync, readFileSync } from "fs";
import { checkDrift } from "@sensei/mcp";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";

export async function status(cwd: string): Promise<void> {
  intro("sensei status");

  const indexDir = senseiPath(cwd);
  const docIndexPath = senseiPath(cwd, "doc-index.json");
  const symbolMapPath = senseiPath(cwd, "symbol-map.json");

  if (!existsSync(indexDir)) {
    log.warn(`Not indexed. Run: sensei init`);
    outro("Done.");
    return;
  }

  const lines: string[] = [];

  // Index info
  if (existsSync(docIndexPath)) {
    const docIndex = JSON.parse(readFileSync(docIndexPath, "utf-8"));
    const commit = docIndex.lastIndexedCommit
      ? docIndex.lastIndexedCommit.slice(0, 8)
      : "unknown (no git)";
    const fileCount = Object.keys(docIndex.files ?? docIndex).length;
    lines.push(`Index: ${fileCount} files at commit ${commit}`);
  }

  if (existsSync(symbolMapPath)) {
    const symbolMap = JSON.parse(readFileSync(symbolMapPath, "utf-8"));
    const exportCount = Object.values(symbolMap as Record<string, unknown[]>)
      .reduce((n, v) => n + (Array.isArray(v) ? v.length : 0), 0);
    lines.push(`Symbols: ${exportCount} exports across ${Object.keys(symbolMap).length} files`);
  }

  // Drift
  const drift = await checkDrift(cwd);
  lines.push(
    drift.drifted.length === 0
      ? "Drift: clean"
      : `Drift: ${drift.drifted.length} doc(s) stale — run sensei drift for details`
  );

  note(lines.join("\n"), "Status");
  outro("Done.");
}
