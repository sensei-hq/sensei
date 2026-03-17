import { stat, readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { execSync } from "child_process";
import { senseiPath, loadSenseiConfig, makeSenseiClient } from "@sensei/shared";

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
  let fingerprints: Record<string, { mtime: number; size: number }> = {};
  let lastIndexedCommit: string | undefined;
  let traceabilityData: Record<string, string[]> = {};

  // Try DB first
  const config = await loadSenseiConfig(repoPath);
  if (config) {
    const client = await makeSenseiClient(repoPath);
    if (client) {
      const { data: repoRow } = await (client as any)
        .schema("sensei").from("repos")
        .select("doc_fingerprints, last_indexed_commit")
        .eq("id", config.repo_id)
        .maybeSingle();
      if (repoRow?.doc_fingerprints) {
        fingerprints = repoRow.doc_fingerprints;
        lastIndexedCommit = repoRow.last_indexed_commit ?? undefined;
      }
      const { data: docsRows } = await (client as any)
        .schema("sensei").from("docs")
        .select("doc_path, covers")
        .eq("repo_id", config.repo_id);
      if (docsRows) {
        for (const row of docsRows) {
          traceabilityData[row.doc_path] = row.covers;
        }
      }
    }
  }

  // Fallback to legacy files
  if (Object.keys(fingerprints).length === 0) {
    const indexPath = senseiPath(repoPath, "doc-index.json");
    if (!existsSync(indexPath)) {
      return { drifted: [], summary: "No index found. Run sensei index first." };
    }
    const raw: DocIndexData = JSON.parse(await readFile(indexPath, "utf-8"));
    fingerprints = (raw.files ?? raw) as Record<string, { mtime: number; size: number }>;
    lastIndexedCommit = raw.lastIndexedCommit as string | undefined;
  }

  if (Object.keys(traceabilityData).length === 0) {
    const traceabilityPath = senseiPath(repoPath, "traceability.json");
    if (existsSync(traceabilityPath)) {
      traceabilityData = JSON.parse(await readFile(traceabilityPath, "utf-8"));
    }
  }

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
    // Git mode: cross-reference changed files with traceability data
    if (Object.keys(traceabilityData).length > 0) {
      for (const [docPath, coveredFiles] of Object.entries(traceabilityData)) {
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
      for (const [docPath] of Object.entries(traceabilityData)) {
        if (changedFiles.has(docPath)) {
          const coveredFiles = traceabilityData[docPath] ?? [];
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
