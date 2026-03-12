import { Database } from "bun:sqlite";
import { existsSync, readFileSync, unlinkSync } from "fs";

interface StoredEvent {
  user_uuid?: string;
  session_id?: string;
  seq?: number | null;
  ts?: number;
  tool?: string;
  phase?: string;
  duration_ms?: number | null;
  success?: boolean | null;
  input?: string | null;
  error?: string | null;
  project_path?: string;
}

function insertRaw(db: Database, e: StoredEvent): void {
  if (!e.ts || !e.tool || (e.phase !== "pre" && e.phase !== "post")) return;

  db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `).run(
    e.user_uuid ?? "",
    e.session_id ?? "",
    e.seq ?? null,
    e.ts,
    e.tool,
    e.phase,
    e.duration_ms ?? null,
    e.success == null ? null : (e.success ? 1 : 0),
    e.input?.slice(0, 2048) ?? null,
    e.error ?? null,
    e.project_path ?? "",
  );
}

export async function drainJsonl(db: Database, jsonlPath: string): Promise<void> {
  if (!existsSync(jsonlPath)) return;

  const content = readFileSync(jsonlPath, "utf8");
  const lines = content.split("\n").filter(l => l.trim().length > 0);

  // Parse JSON first — parse errors are benign skips, not failures
  const validEvents: StoredEvent[] = [];
  for (const line of lines) {
    try {
      validEvents.push(JSON.parse(line) as StoredEvent);
    } catch {
      console.warn("[collector] drain: skipping malformed JSONL line");
    }
  }

  // Insert in a transaction — if any SQLite write fails, rollback and leave file intact
  try {
    db.run("BEGIN");
    for (const event of validEvents) {
      insertRaw(db, event);
    }
    db.run("COMMIT");
  } catch (err) {
    try { db.run("ROLLBACK"); } catch {}
    console.error("[collector] drain: SQLite write failed, leaving JSONL intact:", (err as Error).message);
    return; // File stays intact — caller can retry on next startup
  }

  // All events inserted — delete the JSONL file
  try {
    unlinkSync(jsonlPath);
  } catch (err) {
    console.error("[collector] drain: failed to delete JSONL file:", (err as Error).message);
  }
}
