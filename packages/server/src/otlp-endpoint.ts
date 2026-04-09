export interface OtlpEvent {
  promptId: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  costUsd: number;
  durationMs: number | null;
  model: string | null;
  recordedAt: string;
}

function getAttr(attrs: any[], key: string): any {
  const a = attrs?.find((x: any) => x.key === key);
  if (!a) return null;
  const v = a.value;
  return v.intValue ?? v.doubleValue ?? v.stringValue ?? null;
}

export function parseOtlpBody(body: any): OtlpEvent[] {
  const events: OtlpEvent[] = [];
  const now = new Date().toISOString();

  // Metrics format: resourceMetrics[].scopeMetrics[].metrics[].gauge.dataPoints[]
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

  // Logs format: resourceLogs[].scopeLogs[].logRecords[]
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

export interface OtlpEndpointOptions {
  port?: number;
  dryRun?: boolean;
  repoId: string;
  log?: (msg: string) => void;
}

export function createOtlpEndpoint(opts: OtlpEndpointOptions): { stop: () => void; port: number } {
  const port = opts.port ?? 4318;
  const dryRun = opts.dryRun ?? process.env.SENSEI_OTEL_DRY_RUN === "true";

  const log = opts.log ?? ((msg: string) => console.error(msg));

  const server = Bun.serve({
    port,
    async fetch(req: Request) {
      const url = new URL(req.url);

      if (req.method === "POST" && (url.pathname === "/v1/metrics" || url.pathname === "/v1/logs")) {
        let body: any;
        try { body = await req.json(); }
        catch { return Response.json({ error: "Invalid JSON" }, { status: 400 }); }

        const events = parseOtlpBody(body);

        if (dryRun) {
          for (const event of events) {
            log(`[sensei-otel dry-run] ${JSON.stringify(event)}`);
          }
          return Response.json({ ok: true, received: events.length, mode: "dry-run" });
        }

        if (events.length > 0) {
          log(`[sensei-otel] received ${events.length} event(s): ${events.map(e => e.promptId).join(", ")}`);
        }
        return Response.json({ ok: true, received: events.length });
      }

      return Response.json({ ok: false, error: "Not found" }, { status: 404 });
    },
  });

  const s = server as { stop: () => void; port: number };
  return { stop: () => s.stop(), port: s.port };
}
