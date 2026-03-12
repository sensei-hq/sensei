import { describe, it, expect, afterAll } from "vitest";
import { Database } from "bun:sqlite";
import { createTables } from "./schema.js";
import { startDaemon } from "./daemon.js";

// Use a unique port per test file to avoid conflicts
const PORT = 51800;

describe("startDaemon", () => {
  let db: Database;
  let daemon: { stop: () => void; port: number };

  afterAll(() => daemon?.stop());

  it("starts and responds to GET /health", async () => {
    db = new Database(":memory:");
    createTables(db);
    daemon = await startDaemon(PORT, { db });

    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(200);
    expect(body.ok).toBe(true);
    expect(typeof body.uptime).toBe("number");
  });

  it("POST /event stores a valid pre event in SQLite", async () => {
    const event = {
      user_uuid: "test-uuid",
      session_id: "sess-1",
      ts: Date.now(),
      tool: "Bash",
      phase: "pre",
      project_path: "/projects/foo",
      input: JSON.stringify({ command: "ls" }).slice(0, 2048),
    };

    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(event),
    });
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(200);
    expect(body.ok).toBe(true);

    const row = db.prepare("SELECT * FROM events WHERE session_id = ?").get("sess-1") as Record<string, unknown> | null;
    expect(row).not.toBeNull();
    expect(row!.tool).toBe("Bash");
    expect(row!.phase).toBe("pre");
  });

  it("POST /event stores a post event with success and error", async () => {
    const event = {
      user_uuid: "test-uuid",
      session_id: "sess-2",
      ts: Date.now(),
      tool: "search_index",
      phase: "post",
      success: true,
      project_path: "/projects/foo",
    };

    await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(event),
    });

    const row = db.prepare("SELECT * FROM events WHERE session_id = ?").get("sess-2") as Record<string, unknown> | null;
    expect(row!.success).toBe(1);
    expect(row!.error).toBeNull();
  });

  it("POST /event returns 400 for missing required fields", async () => {
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tool: "Bash" }), // missing phase and ts
    });
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(400);
    expect(body.ok).toBe(false);
    expect(typeof body.error).toBe("string");
  });

  it("POST /event rejects invalid JSON", async () => {
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "not json",
    });
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(400);
    expect(body.ok).toBe(false);
    expect(typeof body.error).toBe("string");
  });

  it("returns 404 for unknown routes", async () => {
    const res = await fetch(`http://localhost:${PORT}/unknown`);
    expect(res.status).toBe(404);
  });
});
