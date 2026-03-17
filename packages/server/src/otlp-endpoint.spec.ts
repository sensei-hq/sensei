import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createOtlpEndpoint } from "./otlp-endpoint.js";

const METRICS_BODY = {
  resourceMetrics: [{
    scopeMetrics: [{
      metrics: [{
        name: "claude_code.api_request",
        gauge: {
          dataPoints: [{
            attributes: [
              { key: "prompt.id", value: { stringValue: "prompt-abc" } },
              { key: "input_tokens", value: { intValue: "1000" } },
              { key: "output_tokens", value: { intValue: "500" } },
              { key: "cache_read_tokens", value: { intValue: "200" } },
              { key: "cache_creation_tokens", value: { intValue: "50" } },
              { key: "cost_usd", value: { doubleValue: 0.015 } },
              { key: "duration_ms", value: { intValue: "2500" } },
              { key: "model", value: { stringValue: "claude-sonnet-4-6" } },
            ],
          }],
        },
      }],
    }],
  }],
};

const LOGS_BODY = {
  resourceLogs: [{
    scopeLogs: [{
      logRecords: [{
        attributes: [
          { key: "event.name", value: { stringValue: "claude_code.api_request" } },
          { key: "prompt.id", value: { stringValue: "prompt-xyz" } },
          { key: "input_tokens", value: { intValue: "800" } },
          { key: "output_tokens", value: { intValue: "300" } },
          { key: "cache_read_tokens", value: { intValue: "0" } },
          { key: "cache_creation_tokens", value: { intValue: "0" } },
          { key: "cost_usd", value: { doubleValue: 0.009 } },
          { key: "duration_ms", value: { intValue: "1800" } },
          { key: "model", value: { stringValue: "claude-haiku-4-5" } },
        ],
      }],
    }],
  }],
};

describe("createOtlpEndpoint", () => {
  let server: ReturnType<typeof createOtlpEndpoint>;
  let port: number;

  beforeEach(() => {
    port = 14400 + Math.floor(Math.random() * 100);
  });

  afterEach(() => {
    server?.stop();
  });

  it("returns 200 and logs event in dry-run mode (metrics format)", async () => {
    const logs: string[] = [];
    const consoleSpy = vi.spyOn(console, "error").mockImplementation((...args) => {
      logs.push(args.join(" "));
    });

    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/v1/metrics`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(METRICS_BODY),
    });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.received).toBe(1);
    expect(body.mode).toBe("dry-run");
    expect(logs.some(l => l.includes("prompt-abc"))).toBe(true);
    consoleSpy.mockRestore();
  });

  it("parses logs format (POST /v1/logs)", async () => {
    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/v1/logs`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(LOGS_BODY),
    });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.received).toBe(1);
  });

  it("ignores non-claude_code.api_request metrics", async () => {
    const otherBody = {
      resourceMetrics: [{
        scopeMetrics: [{
          metrics: [{ name: "some.other.metric", gauge: { dataPoints: [] } }],
        }],
      }],
    };
    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/v1/metrics`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(otherBody),
    });

    const body = await res.json();
    expect(body.received).toBe(0);
  });

  it("writes to supabase in write mode", async () => {
    const inserted: any[] = [];
    const mockClient = {
      from: (table: string) => ({
        insert: (row: any) => { inserted.push({ table, row }); return Promise.resolve({ error: null }); },
        select: () => ({ eq: () => ({ eq: () => ({ lte: () => ({ limit: () => Promise.resolve({ data: [], error: null }) }) }) }) }),
      }),
    };

    server = createOtlpEndpoint({ port, dryRun: false, repoId: "repo-1", supabaseClient: mockClient });
    await fetch(`http://localhost:${port}/v1/metrics`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(METRICS_BODY),
    });

    expect(inserted).toHaveLength(1);
    expect(inserted[0].table).toBe("api_requests");
    expect(inserted[0].row.prompt_id).toBe("prompt-abc");
    expect(inserted[0].row.input_tokens).toBe(1000);
    expect(inserted[0].row.cost_usd).toBe(0.015);
  });

  it("returns 404 for unknown paths", async () => {
    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/unknown`);
    expect(res.status).toBe(404);
  });
});
