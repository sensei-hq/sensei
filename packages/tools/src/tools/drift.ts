import { readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { execFileSync } from "child_process";
import { homedir } from "os";
import { loadSenseiConfig } from "@sensei/shared";

export interface DriftEntry {
  docPath: string;
  reason: "code-changed" | "doc-changed" | "file-deleted" | "raw-modified";
  changedFiles?: string[];
}

export interface DriftResult {
  drifted: DriftEntry[];
  summary: string;
  lastIndexedCommit?: string;
  indexedAt?: string;
}

interface IndexState {
  lastCommit?: string;
  indexedAt?: string;
  repoPath?: string;
}

export async function checkDrift(repoPath: string): Promise<DriftResult> {
  // Resolve repoId from .sensei/config.yaml
  const config = await loadSenseiConfig(repoPath);
  if (!config?.repo_id) {
    return { drifted: [], summary: "Not initialised — run sensei init first." };
  }

  // Read index state written by graph-indexer after each `sensei index`
  const statePath = join(homedir(), ".sensei", "projects", config.repo_id, "index-state.json");
  if (!existsSync(statePath)) {
    return { drifted: [], summary: "No index found. Run sensei index first." };
  }

  const state: IndexState = JSON.parse(await readFile(statePath, "utf-8"));
  const { lastCommit, indexedAt } = state;

  if (!lastCommit) {
    return { drifted: [], summary: "Repository has no git history — drift detection requires git.", indexedAt };
  }

  // Get files changed since the last index commit
  let changedFiles: Set<string>;
  try {
    const out = execFileSync("git", ["diff", `${lastCommit}..HEAD`, "--name-only"], { cwd: repoPath })
      .toString().trim();
    changedFiles = new Set(out ? out.split("\n").filter(Boolean) : []);
  } catch {
    return { drifted: [], summary: "Could not run git diff — check repository state.", lastIndexedCommit: lastCommit, indexedAt };
  }

  if (changedFiles.size === 0) {
    return { drifted: [], summary: "No drift detected. All docs aligned with code.", lastIndexedCommit: lastCommit, indexedAt };
  }

  // Load traceability data (best-effort — written by reindexRepo if present)
  let traceabilityData: Record<string, string[]> = {};
  const traceabilityPath = join(repoPath, ".sensei", "traceability.json");
  if (existsSync(traceabilityPath)) {
    try {
      traceabilityData = JSON.parse(await readFile(traceabilityPath, "utf-8"));
    } catch { /* ignore */ }
  }

  const drifted: DriftEntry[] = [];

  if (Object.keys(traceabilityData).length > 0) {
    // Traceability-aware: cross-reference doc coverage
    for (const [docPath, coveredFiles] of Object.entries(traceabilityData)) {
      const triggeringFiles = coveredFiles.filter(f => changedFiles.has(f));
      if (triggeringFiles.length === 0) continue;
      if (!changedFiles.has(docPath)) {
        drifted.push({ docPath, reason: "code-changed", changedFiles: triggeringFiles });
      }
      // co-change = aligned, not flagged
    }
    for (const [docPath] of Object.entries(traceabilityData)) {
      if (changedFiles.has(docPath)) {
        const codeChanged = (traceabilityData[docPath] ?? []).some(f => changedFiles.has(f));
        if (!codeChanged) drifted.push({ docPath, reason: "doc-changed" });
      }
    }
  } else {
    // No traceability — report changed doc files only
    for (const file of changedFiles) {
      if (file.endsWith(".md") || file.endsWith(".yaml") || file.endsWith(".txt")) {
        drifted.push({ docPath: file, reason: "raw-modified" });
      }
    }
  }

  const since = lastCommit.slice(0, 8);
  const summary = drifted.length === 0
    ? `No drift detected since ${since}.`
    : `${drifted.length} doc(s) drifted since ${since}:\n` +
      drifted.map(d => {
        if (d.reason === "code-changed") return `  ${d.docPath}: code changed — ${d.changedFiles?.join(", ")}`;
        if (d.reason === "doc-changed") return `  ${d.docPath}: doc changed without code change`;
        return `  ${d.docPath}: modified`;
      }).join("\n");

  return { drifted, summary, lastIndexedCommit: lastCommit, indexedAt };
}
