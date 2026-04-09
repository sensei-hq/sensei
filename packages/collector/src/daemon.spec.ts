import { describe, it, expect, afterAll, vi } from "vitest";
import { startDaemon } from "./daemon.js";

// Mock @sensei/server to avoid loading better-sqlite3 in tests
vi.mock("@sensei/server", () => ({
  getActivityLog: vi.fn(() => ({
    logToolCall: vi.fn(),
    logApiRequest: vi.fn(),
  })),
}));

// Mock @sensei/shared to avoid filesystem reads
vi.mock("@sensei/shared", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@sensei/shared")>();
  return { ...actual, loadSenseiConfig: vi.fn().mockResolvedValue(null) };
});

const PORT = 51800;
const OTLP_PORT = 51801;

describe("startDaemon", () => {
  let daemon: { stop: () => void; port: number };
  afterAll(() => daemon?.stop());

  it("starts and responds to GET /health", async () => {
    daemon = await startDaemon(PORT, {});
    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(200);
    expect(body.ok).toBe(true);
  });

  it("POST /event accepts valid event and returns ok", async () => {
    const event = { user_uuid: "u1", session_id: "s1", ts: Date.now(), tool: "Bash", phase: "pre", project_path: "/p" };
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(event),
    });
    expect(res.status).toBe(200);
    const body = await res.json() as Record<string, unknown>;
    expect(body.ok).toBe(true);
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
    daemon = await startDaemon(OTLP_PORT, {});
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

  it("POST /v1/logs with registered repo returns received count", async () => {
    const freshPort = 51803;
    const freshDaemon = await startDaemon(freshPort, {});
    try {
      await fetch(`http://localhost:${freshPort}/otlp/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ repoId: "repo-xyz", repoPath: "/fake/path" }),
      });

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
      expect(body.received).toBe(1);
    } finally {
      freshDaemon.stop();
    }
  });
});
