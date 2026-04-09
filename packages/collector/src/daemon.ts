import { loadSenseiConfig } from "@sensei/shared";
import { getActivityLog } from "@sensei/server";

export interface DaemonOptions {
  repoPath?: string;               // used to resolve config
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

/** Cache: project_path → repoId (loaded from .sensei/config.yaml) */
const repoIdCache = new Map<string, string>();

async function resolveRepoId(projectPath: string): Promise<string | null> {
  if (!projectPath) return null;
  const cached = repoIdCache.get(projectPath);
  if (cached) return cached;
  try {
    const config = await loadSenseiConfig(projectPath);
    if (config?.repo_id) {
      repoIdCache.set(projectPath, config.repo_id);
      return config.repo_id;
    }
  } catch { /* ignore */ }
  return null;
}

export async function startDaemon(port: number, opts: DaemonOptions = {}): Promise<Daemon> {
  // latest registration: { repoId, repoPath }
  let activeRepo: { repoId: string; repoPath: string } | null = null;

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

        // Primary sink: ActivityLog SQLite
        const repoId = await resolveRepoId(body.project_path ?? "");
        if (repoId) {
          try {
            const log = getActivityLog(repoId);
            log.logToolCall({
              sessionId: body.session_id ?? null,
              tool: body.tool,
              phase: body.phase,
              success: body.success ?? null,
              durationMs: body.duration_ms ?? null,
              input: body.input ?? null,
              error: body.error ?? null,
              ts: new Date(body.ts).toISOString(),
            });
          } catch (err) {
            console.error("[collector] ActivityLog write error:", (err as Error).message);
          }
        }

        return Response.json({ ok: true });
      }

      if (req.method === "POST" && url.pathname === "/otlp/register") {
        let body: unknown;
        try {
          body = await req.json();
        } catch {
          return Response.json({ ok: false, error: "invalid JSON" }, { status: 400 });
        }
        const { repoId, repoPath } = body as { repoId: string; repoPath: string };
        activeRepo = { repoId, repoPath };
        return Response.json({ ok: true });
      }

      if (req.method === "POST" && (url.pathname === "/v1/metrics" || url.pathname === "/v1/logs")) {
        let body: unknown;
        try {
          body = await req.json();
        } catch {
          return Response.json({ ok: true, received: 0, note: "invalid JSON" });
        }
        const events = parseOtlpBody(body);
        if (events.length === 0) return Response.json({ ok: true, received: 0 });

        // Primary sink: ActivityLog SQLite
        if (activeRepo?.repoId) {
          try {
            const log = getActivityLog(activeRepo.repoId);
            for (const event of events) {
              log.logApiRequest({
                sessionId: null,
                promptId: event.promptId,
                inputTokens: event.inputTokens,
                outputTokens: event.outputTokens,
                cacheReadTokens: event.cacheReadTokens,
                cacheCreationTokens: event.cacheCreationTokens,
                costUsd: event.costUsd,
                durationMs: event.durationMs,
                model: event.model,
                recordedAt: event.recordedAt,
              });
            }
          } catch (err) {
            console.error("[collector] ActivityLog OTLP write error:", (err as Error).message);
          }
        }

        return Response.json({ ok: true, received: events.length });
      }

      return Response.json({ ok: false, error: "not found" }, { status: 404 });
    },
  }) as { stop: () => void; port: number };

  return { stop: () => server.stop(), port: server.port };
}
