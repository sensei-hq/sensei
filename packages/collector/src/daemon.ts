import type { SupabaseClient } from "@supabase/supabase-js";
import { appendFileSync } from "fs";
import { makeSenseiClient } from "@sensei/shared";
import { writeEventToSupabase } from "./supabase-writer.js";
import { drainJsonl } from "./drain.js";

export interface DaemonOptions {
  supabaseClient?: SupabaseClient; // injectable for tests
  jsonlPath?: string;              // path to JSONL fallback file; drained on startup
  repoPath?: string;               // used to resolve Supabase config
  otlpSupabaseClient?: any;        // injected in tests; otherwise loaded from repoPath on register
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

function getAttr(attrs: any[], key: string): any {
  const a = attrs?.find((x: any) => x.key === key);
  if (!a) return null;
  const v = a.value;
  return v.intValue ?? v.doubleValue ?? v.stringValue ?? null;
}

function parseOtlpBody(body: any): Array<{
  promptId: string; inputTokens: number; outputTokens: number;
  cacheReadTokens: number; cacheCreationTokens: number;
  costUsd: number; durationMs: number | null; model: string | null; recordedAt: string;
}> {
  const events: any[] = [];
  const now = new Date().toISOString();

  for (const rm of body.resourceMetrics ?? []) {
    for (const sm of rm.scopeMetrics ?? []) {
      for (const m of sm.metrics ?? []) {
        if (m.name !== "claude_code.api_request") continue;
        const dataPoints = m.gauge?.dataPoints ?? m.sum?.dataPoints ?? [];
        for (const dp of dataPoints) {
          const attrs = dp.attributes ?? [];
          const promptId = getAttr(attrs, "prompt.id");
          if (!promptId) continue;
          events.push({
            promptId: String(promptId),
            inputTokens: Number(getAttr(attrs, "input_tokens") ?? 0),
            outputTokens: Number(getAttr(attrs, "output_tokens") ?? 0),
            cacheReadTokens: Number(getAttr(attrs, "cache_read_tokens") ?? 0),
            cacheCreationTokens: Number(getAttr(attrs, "cache_creation_tokens") ?? 0),
            costUsd: Number(getAttr(attrs, "cost_usd") ?? 0),
            durationMs: getAttr(attrs, "duration_ms") != null ? Number(getAttr(attrs, "duration_ms")) : null,
            model: getAttr(attrs, "model") ? String(getAttr(attrs, "model")) : null,
            recordedAt: now,
          });
        }
      }
    }
  }

  for (const rl of body.resourceLogs ?? []) {
    for (const sl of rl.scopeLogs ?? []) {
      for (const lr of sl.logRecords ?? []) {
        const attrs = lr.attributes ?? [];
        const eventName = getAttr(attrs, "event.name");
        if (eventName !== "claude_code.api_request") continue;
        const promptId = getAttr(attrs, "prompt.id");
        if (!promptId) continue;
        events.push({
          promptId: String(promptId),
          inputTokens: Number(getAttr(attrs, "input_tokens") ?? 0),
          outputTokens: Number(getAttr(attrs, "output_tokens") ?? 0),
          cacheReadTokens: Number(getAttr(attrs, "cache_read_tokens") ?? 0),
          cacheCreationTokens: Number(getAttr(attrs, "cache_creation_tokens") ?? 0),
          costUsd: Number(getAttr(attrs, "cost_usd") ?? 0),
          durationMs: getAttr(attrs, "duration_ms") != null ? Number(getAttr(attrs, "duration_ms")) : null,
          model: getAttr(attrs, "model") ? String(getAttr(attrs, "model")) : null,
          recordedAt: now,
        });
      }
    }
  }

  return events;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let supabaseClient: any = opts.supabaseClient ?? null;
  if (!supabaseClient) {
    makeSenseiClient(opts.repoPath ?? process.cwd()).then(c => { supabaseClient = c; });
  }

  // latest registration: { repoId, repoPath, client }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let activeRepo: { repoId: string; repoPath: string; client: any } | null =
    opts.otlpSupabaseClient ? { repoId: "", repoPath: "", client: opts.otlpSupabaseClient } : null;

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

      if (req.method === "POST" && url.pathname === "/otlp/register") {
        let body: unknown;
        try {
          body = await req.json();
        } catch {
          return Response.json({ ok: false, error: "invalid JSON" }, { status: 400 });
        }
        const { repoId, repoPath } = body as { repoId: string; repoPath: string };
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let client: any = activeRepo?.client ?? null; // preserve injected client as fallback
        try {
          const loaded = await makeSenseiClient(repoPath);
          if (loaded) client = loaded;
        } catch { /* client unavailable — keep existing */ }
        activeRepo = { repoId, repoPath, client };
        return Response.json({ ok: true });
      }

      if (req.method === "POST" && (url.pathname === "/v1/metrics" || url.pathname === "/v1/logs")) {
        let body: unknown;
        try {
          body = await req.json();
        } catch {
          return Response.json({ ok: true, received: 0, note: "invalid JSON" });
        }
        if (!activeRepo || !activeRepo.client) {
          return Response.json({ ok: true, received: 0, note: "no repo registered" });
        }
        const events = parseOtlpBody(body);
        for (const event of events) {
          await activeRepo.client.from("api_requests").insert({
            repo_id: activeRepo.repoId,
            task_session_id: null,
            prompt_id: event.promptId,
            input_tokens: event.inputTokens,
            output_tokens: event.outputTokens,
            cache_read_tokens: event.cacheReadTokens,
            cache_creation_tokens: event.cacheCreationTokens,
            cost_usd: event.costUsd,
            duration_ms: event.durationMs,
            model: event.model,
            recorded_at: event.recordedAt,
          });
        }
        return Response.json({ ok: true, received: events.length });
      }

      return Response.json({ ok: false, error: "not found" }, { status: 404 });
    },
  }) as { stop: () => void; port: number };

  return { stop: () => server.stop(), port: server.port };
}
