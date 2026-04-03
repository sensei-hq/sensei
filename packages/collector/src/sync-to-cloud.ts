// packages/collector/src/sync-to-cloud.ts
// Syncs PII-scrubbed session summaries to the platform API after checkpoint().
import { createHash } from "crypto";
import { readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import yaml from "js-yaml";

const CRED_PATH = join(homedir(), ".config", "sensei", "credentials.yaml");
const MAX_RETRIES = 3;

export interface SyncPayload {
  sessionId: string;
  repoSlug: string;        // SHA-256 hash of repo name + account salt
  stack: string[];
  ftrScore: number | null;
  tokenCost: number | null;
  durationMs: number | null;
  toolCallCount: number;
  errorCount: number;
  snapshotCount: number;
  completedCleanly: boolean;
  recordedAt: string;
}

interface StoredCredentials {
  access_token?: string;
  account_id?: string;
}

async function loadCredentials(): Promise<StoredCredentials | null> {
  try {
    const raw = await readFile(CRED_PATH, "utf-8");
    return yaml.load(raw) as StoredCredentials;
  } catch {
    return null;
  }
}

function scrubRepoName(repoName: string, accountId: string): string {
  return createHash("sha256")
    .update(`${accountId}:${repoName}`)
    .digest("hex")
    .slice(0, 16);
}

async function postWithRetry(
  url: string,
  payload: unknown,
  token: string,
  retries = MAX_RETRIES,
): Promise<void> {
  let lastErr: Error | null = null;
  let delay = 500;

  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      const res = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(payload),
        signal: AbortSignal.timeout(10_000),
      });

      if (res.ok) return;
      if (res.status >= 400 && res.status < 500) return; // 4xx: drop, don't retry
      lastErr = new Error(`sync: HTTP ${res.status}`);
    } catch (err) {
      lastErr = err as Error;
    }

    if (attempt < retries) {
      await new Promise(r => setTimeout(r, delay));
      delay *= 2;
    }
  }

  // Silent failure — never let sync errors surface to the user
  if (process.env.SENSEI_SYNC_DEBUG) {
    console.error(`[sensei sync] failed after ${retries} retries: ${lastErr?.message}`);
  }
}

export interface SessionSummary {
  sessionId: string;
  repoName: string;
  stack: string[];
  ftrScore: number | null;
  tokenCost: number | null;
  durationMs: number | null;
  toolCallCount: number;
  errorCount: number;
  snapshotCount: number;
  completedCleanly: boolean;
}

/**
 * Sync a PII-scrubbed session summary to the platform API.
 * Silent no-op if credentials are not present (local-only mode).
 */
export async function syncToCloud(
  session: SessionSummary,
  platformUrl?: string,
): Promise<void> {
  const creds = await loadCredentials();
  if (!creds?.access_token || !creds?.account_id) return; // local-only mode

  const base = platformUrl ?? process.env.SENSEI_PLATFORM_URL ?? "https://app.sensei.dev";

  const payload: SyncPayload = {
    sessionId: session.sessionId,
    repoSlug: scrubRepoName(session.repoName, creds.account_id),
    stack: session.stack,
    ftrScore: session.ftrScore,
    tokenCost: session.tokenCost,
    durationMs: session.durationMs,
    toolCallCount: session.toolCallCount,
    errorCount: session.errorCount,
    snapshotCount: session.snapshotCount,
    completedCleanly: session.completedCleanly,
    recordedAt: new Date().toISOString(),
  };

  await postWithRetry(`${base}/sync/session`, payload, creds.access_token);
}
