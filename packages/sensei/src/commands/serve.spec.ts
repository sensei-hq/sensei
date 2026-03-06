import { describe, it, expect, afterAll } from "vitest";
import { createReportServer } from "./serve.js";
import { tmpdir } from "os";
import { join } from "path";

const PORT = 17744; // non-default to avoid conflicts
const DB_PATH = join(tmpdir(), `sensei-test-${Date.now()}.db`);

describe("createReportServer", () => {
  let server: { stop: () => void };

  afterAll(() => server?.stop());

  it("returns health ok", async () => {
    server = await createReportServer({ port: PORT, dbPath: DB_PATH });
    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json();
    expect(body).toEqual({ ok: true });
  });

  it("accepts POST /reports and returns id", async () => {
    const report = { id: "test-123", timestamp: "2026-03-06T00:00:00Z", scenario: {} };
    const res = await fetch(`http://localhost:${PORT}/reports`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report),
    });
    const body = await res.json();
    expect(body.ok).toBe(true);
    expect(body.id).toBe("test-123");
  });

  it("returns 404 for unknown routes", async () => {
    const res = await fetch(`http://localhost:${PORT}/unknown`);
    expect(res.status).toBe(404);
  });
});
