import { Database } from "bun:sqlite";
import { mkdirSync } from "fs";
import { dirname } from "path";
import { createTables } from "./schema.js";

export interface DaemonOptions {
  db?: Database;          // injectable for tests; if omitted, opened from dbPath
  dbPath?: string;        // ignored when db is provided
  jsonlPath?: string;     // path to JSONL fallback file; drained on startup (wired in Task 5)
}

export interface Daemon {
  stop: () => void;
  port: number;
}

interface EventPayload {
  user_uuid?: string;
  session_id?: string;
  seq?: number | null;
  ts: number;
  tool: string;
  phase: "pre" | "post";
  duration_ms?: number | null;
  success?: boolean | null;
  input?: string | null;
  error?: string | null;
  project_path?: string;
}

function isValidEvent(body: unknown): body is EventPayload {
  if (!body || typeof body !== "object") return false;
  const b = body as Record<string, unknown>;
  return (
    typeof b.ts === "number" &&
    typeof b.tool === "string" && b.tool.length > 0 &&
    (b.phase === "pre" || b.phase === "post")
  );
}

function insertEvent(db: Database, e: EventPayload): void {
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

  // Upsert into projects
  const now = Date.now();
  db.prepare(`
    INSERT INTO projects (path, first_seen, last_seen) VALUES (?, ?, ?)
    ON CONFLICT(path) DO UPDATE SET last_seen = excluded.last_seen
  `).run(e.project_path ?? "", now, now);
}

export async function startDaemon(port: number, opts: DaemonOptions = {}): Promise<Daemon> {
  const db = opts.db ?? (() => {
    mkdirSync(dirname(opts.dbPath!), { recursive: true });
    return new Database(opts.dbPath!);
  })();

  createTables(db);

  const startedAt = Date.now();

  const server = Bun.serve({
    port,
    async fetch(req: Request): Promise<Response> {
      const url = new URL(req.url);

      if (req.method === "GET" && url.pathname === "/health") {
        return Response.json({ ok: true, uptime: Date.now() - startedAt });
      }

      if (req.method === "POST" && url.pathname === "/event") {
        let body: unknown;
        try {
          body = await req.json();
        } catch {
          return Response.json({ ok: false, error: "invalid JSON" }, { status: 400 });
        }
        if (!isValidEvent(body)) {
          return Response.json({ ok: false, error: "invalid payload: ts, tool, and phase are required" }, { status: 400 });
        }
        try {
          insertEvent(db, body);
          return Response.json({ ok: true });
        } catch (err) {
          console.error("[collector] SQLite write error:", (err as Error).message);
          return Response.json({ ok: false, error: "write failed" }, { status: 500 });
        }
      }

      return Response.json({ ok: false, error: "not found" }, { status: 404 });
    },
  }) as { stop: () => void; port: number };

  return { stop: () => server.stop(), port: server.port };
}
