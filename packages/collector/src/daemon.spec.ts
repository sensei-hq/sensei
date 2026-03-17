import { describe, it, expect, afterAll, vi } from "vitest";
import { startDaemon } from "./daemon.js";

const PORT = 51800;
const OTLP_PORT = 51801;

// Mock Supabase client
const insertedEvents: any[] = [];
const mockInsert = vi.fn((data: any) => { insertedEvents.push(data); return { error: null }; });
const mockFrom = vi.fn(() => ({ insert: mockInsert }));
const mockClient = { from: mockFrom } as any;

describe("startDaemon", () => {
  let daemon: { stop: () => void; port: number };
  afterAll(() => daemon?.stop());

  it("starts and responds to GET /health", async () => {
    daemon = await startDaemon(PORT, { supabaseClient: mockClient });
    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(200);
    expect(body.ok).toBe(true);
  });

  it("POST /event writes to Supabase", async () => {
    insertedEvents.length = 0;
    const event = { user_uuid: "u1", session_id: "s1", ts: Date.now(), tool: "Bash", phase: "pre", project_path: "/p" };
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(event),
    });
    expect(res.status).toBe(200);
    expect(mockInsert).toHaveBeenCalled();
  });

  it("POST /event returns 400 for missing required fields", async () => {
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ tool: "Bash" }),
    });
    expect(res.status).toBe(400);
  });

  it("POST /event rejects invalid JSON", async () => {
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: "not json",
    });
    expect(res.status).toBe(400);
  });

  it("returns 404 for unknown routes", async () => {
    const res = await fetch(`http://localhost:${PORT}/unknown`);
    expect(res.status).toBe(404);
  });
});

describe("startDaemon OTLP routes", () => {
  let daemon: { stop: () => void; port: number };
  afterAll(() => daemon?.stop());

  it("POST /otlp/register stores repo registration", async () => {
    daemon = await startDaemon(OTLP_PORT, { supabaseClient: mockClient });
    const res = await fetch(`http://localhost:${OTLP_PORT}/otlp/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ repoId: "repo-123", repoPath: "/fake/path" }),
    });
    expect(res.status).toBe(200);
    const body = await res.json() as any;
    expect(body.ok).toBe(true);
  });

  it("POST /v1/logs returns ok with no repo registered (graceful no-op)", async () => {
    // Start a fresh daemon with no activeRepo
    const freshPort = 51802;
    const freshDaemon = await startDaemon(freshPort, {});
    try {
      const logsPayload = {
        resourceLogs: [{
          scopeLogs: [{
            logRecords: [{
              attributes: [
                { key: "event.name", value: { stringValue: "claude_code.api_request" } },
                { key: "prompt.id", value: { stringValue: "prompt-abc" } },
                { key: "input_tokens", value: { intValue: 100 } },
                { key: "output_tokens", value: { intValue: 50 } },
                { key: "cache_read_tokens", value: { intValue: 0 } },
                { key: "cache_creation_tokens", value: { intValue: 0 } },
                { key: "cost_usd", value: { doubleValue: 0.001 } },
              ],
            }],
          }],
        }],
      };
      const res = await fetch(`http://localhost:${freshPort}/v1/logs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(logsPayload),
      });
      expect(res.status).toBe(200);
      const body = await res.json() as any;
      expect(body.ok).toBe(true);
    } finally {
      freshDaemon.stop();
    }
  });

  it("POST /v1/logs inserts OTLP events when repo registered via otlpSupabaseClient", async () => {
    const otlpInserted: any[] = [];
    const otlpInsert = vi.fn((data: any) => { otlpInserted.push(data); return { error: null }; });
    const otlpFrom = vi.fn(() => ({ insert: otlpInsert }));
    const otlpClient = { from: otlpFrom } as any;

    const otlpDaemon = await startDaemon(51803, { otlpSupabaseClient: otlpClient });
    // Update activeRepo repoId by registering
    await fetch(`http://localhost:51803/otlp/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ repoId: "repo-xyz", repoPath: "/fake/path" }),
    });

    // Post a logs payload
    const logsPayload = {
      resourceLogs: [{
        scopeLogs: [{
          logRecords: [{
            attributes: [
              { key: "event.name", value: { stringValue: "claude_code.api_request" } },
              { key: "prompt.id", value: { stringValue: "prompt-abc" } },
              { key: "input_tokens", value: { intValue: 100 } },
              { key: "output_tokens", value: { intValue: 50 } },
              { key: "cache_read_tokens", value: { intValue: 0 } },
              { key: "cache_creation_tokens", value: { intValue: 0 } },
              { key: "cost_usd", value: { doubleValue: 0.001 } },
            ],
          }],
        }],
      }],
    };
    const res = await fetch(`http://localhost:51803/v1/logs`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(logsPayload),
    });
    expect(res.status).toBe(200);
    const body = await res.json() as any;
    expect(body.ok).toBe(true);
    expect(body.received).toBe(1);
    otlpDaemon.stop();
  });
});
