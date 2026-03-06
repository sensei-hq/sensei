import { stat, readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { execSync } from "child_process";

export interface DriftEntry {
  docPath: string;
  reason: "code-changed" | "doc-changed" | "file-deleted" | "raw-modified";
  changedFiles?: string[];
}

export interface DriftResult {
  drifted: DriftEntry[];
  summary: string;
  lastIndexedCommit?: string;
}

interface DocIndexData {
  lastIndexedCommit?: string;
  files?: Record<string, { mtime: number; size: number }>;
  // old flat format
  [key: string]: unknown;
}

export async function checkDrift(repoPath: string): Promise<DriftResult> {
  const indexPath = join(repoPath, ".index/doc-index.json");
  if (!existsSync(indexPath)) {
    return { drifted: [], summary: "No doc-index.json found. Run sensei index first." };
  }

  const raw: DocIndexData = JSON.parse(await readFile(indexPath, "utf-8"));

  // Support both new schema (with files key) and old flat schema
  const fingerprints: Record<string, { mtime: number; size: number }> =
    raw.files ?? (raw as Record<string, { mtime: number; size: number }>);
  const lastIndexedCommit = raw.lastIndexedCommit as string | undefined;

  const drifted: DriftEntry[] = [];

  // Determine changed files via git diff or mtime fallback
  const isGit = existsSync(join(repoPath, ".git"));
  let changedFiles = new Set<string>();

  if (isGit && lastIndexedCommit) {
    try {
      const changed = execSync(`git diff ${lastIndexedCommit}..HEAD --name-only`, { cwd: repoPath })
        .toString().trim().split("\n").filter(Boolean);
      changedFiles = new Set(changed);
    } catch { /* fall through to mtime */ }
  }

  if (changedFiles.size === 0) {
    // Mtime fallback: check each fingerprinted file
    for (const [file, fingerprint] of Object.entries(fingerprints)) {
      const fullPath = join(repoPath, file);
      if (!existsSync(fullPath)) {
        drifted.push({ docPath: file, reason: "file-deleted" });
        continue;
      }
      const current = await stat(fullPath);
      if (Math.abs(current.mtimeMs - fingerprint.mtime) > 1000 || current.size !== fingerprint.size) {
        drifted.push({ docPath: file, reason: "raw-modified" });
      }
    }
  } else {
    // Git mode: cross-reference changed files with traceability matrix
    const traceabilityPath = join(repoPath, ".index/traceability.json");
    if (existsSync(traceabilityPath)) {
      const traceability: Record<string, string[]> = JSON.parse(
        await readFile(traceabilityPath, "utf-8")
      );

      for (const [docPath, coveredFiles] of Object.entries(traceability)) {
        const triggeringFiles = coveredFiles.filter(f => changedFiles.has(f));
        if (triggeringFiles.length === 0) continue;

        const docChanged = changedFiles.has(docPath);
        if (!docChanged) {
          drifted.push({
            docPath,
            reason: "code-changed",
            changedFiles: triggeringFiles,
          });
        }
        // co-change (both doc and code changed) = aligned, not flagged
      }

      // Also flag docs that changed without their code changing
      for (const [docPath] of Object.entries(traceability)) {
        if (changedFiles.has(docPath)) {
          const coveredFiles = traceability[docPath] ?? [];
          const codeChanged = coveredFiles.some(f => changedFiles.has(f));
          if (!codeChanged) {
            drifted.push({ docPath, reason: "doc-changed" });
          }
        }
      }
    } else {
      // No traceability — report raw changed doc files only
      for (const file of changedFiles) {
        if (file.endsWith(".md") || file.endsWith(".yaml") || file.endsWith(".txt")) {
          drifted.push({ docPath: file, reason: "raw-modified" });
        }
      }
    }
  }

  const commit = lastIndexedCommit ? ` since ${lastIndexedCommit.slice(0, 8)}` : "";
  const summary = drifted.length === 0
    ? `No drift detected. All docs aligned with code${commit}.`
    : `${drifted.length} doc(s) drifted${commit}:\n` +
      drifted.map(d => {
        if (d.reason === "code-changed") return `${d.docPath}: code changed — ${d.changedFiles?.join(", ")}`;
        if (d.reason === "doc-changed") return `${d.docPath}: doc changed without code change`;
        if (d.reason === "file-deleted") return `${d.docPath}: deleted (was in index)`;
        return `${d.docPath}: modified since last index`;
      }).join("\n");

  return { drifted, summary, lastIndexedCommit };
}
