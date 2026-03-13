import type { SupabaseClient } from "@supabase/supabase-js";
import { appendFileSync } from "fs";
import { makeSenseiClient } from "@sensei/shared";
import { writeEventToSupabase } from "./supabase-writer.js";
import { drainJsonl } from "./drain.js";

export interface DaemonOptions {
  supabaseClient?: SupabaseClient; // injectable for tests
  jsonlPath?: string;              // path to JSONL fallback file; drained on startup
  repoPath?: string;               // used to resolve Supabase config
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

export async function startDaemon(port: number, opts: DaemonOptions = {}): Promise<Daemon> {
  let supabaseClient: SupabaseClient | null = opts.supabaseClient ?? null;
  if (!supabaseClient) {
    makeSenseiClient(opts.repoPath ?? process.cwd()).then(c => { supabaseClient = c; });
  }

  if (opts.jsonlPath && supabaseClient) {
    await drainJsonl(supabaseClient, opts.jsonlPath);
  }

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

        if (supabaseClient) {
          await writeEventToSupabase(supabaseClient, {
            user_uuid:    body.user_uuid ?? "",
            session_id:   body.session_id ?? null,
            repo_id:      null,
            phase:        body.phase,
            tool:         body.tool,
            project_path: body.project_path ?? "",
            input:        body.input ? (() => { try { return JSON.parse(body.input!); } catch { return null; } })() : null,
            ts:           new Date(body.ts),
            seq:          body.seq ?? null,
            duration_ms:  body.duration_ms ?? null,
            success:      body.success ?? null,
            error:        body.error ?? null,
          });
          return Response.json({ ok: true });
        }

        if (opts.jsonlPath) {
          try {
            appendFileSync(opts.jsonlPath, JSON.stringify(body) + "\n");
            return Response.json({ ok: true });
          } catch (err) {
            console.error("[collector] JSONL write error:", (err as Error).message);
            return Response.json({ ok: false, error: "write failed" }, { status: 500 });
          }
        }

        return Response.json({ ok: false, error: "no storage configured" }, { status: 503 });
      }

      return Response.json({ ok: false, error: "not found" }, { status: 404 });
    },
  }) as { stop: () => void; port: number };

  return { stop: () => server.stop(), port: server.port };
}
