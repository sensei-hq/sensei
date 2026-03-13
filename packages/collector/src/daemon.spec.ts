import { describe, it, expect, afterAll, vi } from "vitest";
import { startDaemon } from "./daemon.js";

const PORT = 51800;

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
