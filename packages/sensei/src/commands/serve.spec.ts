import { describe, it, expect, afterAll } from "vitest";
import { createReportServer } from "./serve.js";
import { tmpdir } from "os";
import { join } from "path";

const PORT = 17744; // non-default to avoid conflicts
const DB_PATH = join(tmpdir(), `sensei-test-${Date.now()}.db`);

describe("createReportServer", () => {
  let server: { stop: () => void; port: number };

  afterAll(() => server?.stop());

  it("returns health ok", async () => {
    server = await createReportServer({ port: PORT, dbPath: DB_PATH, isAvailableFn: async () => ({ ollamaRunning: false, ollamaModel: false }) });
    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json() as Record<string, unknown>;
    expect(body.ok).toBe(true);
    expect(body.ollamaRunning).toBe(false);
    expect(body.ollamaModel).toBe(false);
    expect(body.backend).toBe("none");
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

describe("GET /setup/status", () => {
  it("returns SetupStatus shape", async () => {
    const { stop, port } = await createReportServer({ port: 17745 });
    try {
      const res = await fetch(`http://localhost:${port}/setup/status`);
      const data = await res.json() as Record<string, unknown>;
      expect(res.status).toBe(200);
      expect(typeof data.ollamaBinary).toBe("boolean");
      expect(typeof data.diskFreeGB).toBe("number");
      expect(typeof data.ollamaModelName).toBe("string");
    } finally {
      stop();
    }
  });
});

describe("POST /analyze", () => {
  it("returns 400 when filePath is missing", async () => {
    const { stop, port } = await createReportServer({ port: 17746 });
    try {
      const res = await fetch(`http://localhost:${port}/analyze`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: "hello" }),
      });
      expect(res.status).toBe(400);
    } finally {
      stop();
    }
  });

  it("returns FileAnalysis shape (fallback when Ollama unavailable)", async () => {
    const { stop, port } = await createReportServer({ port: 17747 });
    try {
      const res = await fetch(`http://localhost:${port}/analyze`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ filePath: "src/foo.ts", content: "export const x = 1;" }),
      });
      expect(res.status).toBe(200);
      const data = await res.json() as Record<string, unknown>;
      expect(typeof data.path).toBe("string");
      expect(Array.isArray(data.symbols)).toBe(true);
      expect(typeof data.summary).toBe("string");
    } finally {
      stop();
    }
  });
});

describe("GET /health (extended)", () => {
  it("includes ollamaRunning field", async () => {
    const { stop, port } = await createReportServer({ port: 17748, isAvailableFn: async () => ({ ollamaRunning: false, ollamaModel: false }) });
    try {
      const res = await fetch(`http://localhost:${port}/health`);
      const data = await res.json() as Record<string, unknown>;
      expect(res.status).toBe(200);
      expect(data.ok).toBe(true);
      expect(data.ollamaRunning).toBe(false);
      expect(data.ollamaModel).toBe(false);
      expect(data.backend).toBe("none");
    } finally {
      stop();
    }
  });
});
