import { stat, readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";

export interface DriftResult {
  drifted: string[];
  summary: string;
}

export async function checkDrift(repoPath: string): Promise<DriftResult> {
  const indexPath = join(repoPath, ".index/doc-index.json");
  if (!existsSync(indexPath)) {
    return { drifted: [], summary: "No doc-index.json found. Run reindex_repo first." };
  }

  const stored: Record<string, { mtime: number; size: number }> = JSON.parse(
    await readFile(indexPath, "utf-8")
  );

  const drifted: string[] = [];

  for (const [file, fingerprint] of Object.entries(stored)) {
    const fullPath = join(repoPath, file);
    if (!existsSync(fullPath)) {
      drifted.push(`${file}: deleted (was in index)`);
      continue;
    }
    const current = await stat(fullPath);
    if (Math.abs(current.mtimeMs - fingerprint.mtime) > 1000 || current.size !== fingerprint.size) {
      drifted.push(`${file}: modified since last index`);
    }
  }

  const summary = drifted.length === 0
    ? "No drift detected. All indexed docs match current state."
    : `${drifted.length} file(s) drifted since last index:\n${drifted.join("\n")}`;

  return { drifted, summary };
}
