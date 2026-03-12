import { describe, it, expect } from "vitest";
import { Database } from "bun:sqlite";
import { createTables } from "./schema.js";
import { queryStats } from "./stats.js";

function seedDb(db: Database) {
  const insert = db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);

  const now = Date.now();
  const day = 24 * 60 * 60 * 1000;
  const recent = now - 3 * day; // 3 days ago — within 7-day window
  const old    = now - 10 * day; // 10 days ago — outside 7-day window

  // Recent events (within 7 days)
  insert.run("u1", "sess-a", 1, recent, "search_index", "post", 120, 1, null, null, "/proj");
  insert.run("u1", "sess-a", 2, recent + 1000, "search_index", "post", 200, 1, null, null, "/proj");
  insert.run("u1", "sess-a", 3, recent + 2000, "Bash", "post", 300, 0, null, "exit 1", "/proj");
  insert.run("u1", "sess-b", 1, recent + 3000, "Read", "post", 50, 1, null, null, "/proj");

  // Old events (outside 7 days)
  insert.run("u1", "sess-c", 1, old, "search_index", "post", 100, 1, null, null, "/proj");
}

describe("queryStats", () => {
  it("default 7-day summary counts only recent events", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    expect(result.total_calls).toBe(4); // 4 recent post events
    expect(result.tools.some(t => t.name === "search_index")).toBe(true);
    expect(result.sessions).toBe(2); // sess-a, sess-b
    expect(result.period.from).toBeDefined();
  });

  it("--all includes events older than 7 days", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { all: true });
    expect(result.total_calls).toBe(5); // includes the old one
  });

  it("--tool returns stats for one tool only", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { tool: "search_index" });
    expect(result.tool).toBeDefined();
    expect(result.tool!.name).toBe("search_index");
    expect(result.tool!.calls).toBe(2); // 2 recent search_index events
    expect(result.tool!.success_rate).toBe(1.0);
    expect(result.tool!.avg_duration_ms).toBe(160); // (120+200)/2
    expect(result.tool!.last_called).toBeTypeOf("number");
  });

  it("--session returns chronological events for that session", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { session: "sess-a" });
    expect(result.events).toBeDefined();
    expect(result.events!.length).toBe(3);
    expect(result.events![0].tool).toBe("search_index");
  });

  it("--session returns events for sessions older than 7 days (no date filter)", () => {
    const db = new Database(":memory:");
    createTables(db);
    // seed an event 30 days ago — outside any default window
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;
    db.prepare(`INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
      .run("u1", "old-sess", 1, thirtyDaysAgo, "Read", "post", 40, 1, null, null, "/proj");

    const result = queryStats(db, { session: "old-sess" });
    expect(result.events).toBeDefined();
    expect(result.events!.length).toBe(1); // must not be filtered out by 7-day default
  });

  it("--session period.from reflects the earliest event timestamp", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { session: "sess-a" });
    expect(result.events).toBeDefined();
    expect(result.period.from).toMatch(/^\d{4}-\d{2}-\d{2}$/); // YYYY-MM-DD
    expect(result.period.from).not.toBe("");
  });

  it("--since excludes events before the given date", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    const result = queryStats(db, { since: yesterday });
    // All seed events are 3+ days ago, so nothing falls within "since yesterday"
    expect(result.total_calls).toBe(0);
  });

  it("--since includes events on or after the given date", () => {
    const db = new Database(":memory:");
    createTables(db);
    // Seed one event right now (not via seedDb which uses old timestamps)
    db.prepare(`INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
      .run("u1", "now-sess", 1, Date.now(), "Read", "post", 50, 1, null, null, "/proj");

    const today = new Date().toISOString().slice(0, 10);
    const result = queryStats(db, { since: today });
    expect(result.total_calls).toBeGreaterThan(0);
  });

  it("--since combined with --tool filters by both date and tool", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db); // has search_index events 3 days ago and one 10 days ago

    // Filter: only search_index events from the last 5 days
    const fiveDaysAgo = new Date(Date.now() - 5 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    const result = queryStats(db, { tool: "search_index", since: fiveDaysAgo });
    // Should find 2 recent search_index events but NOT the old one
    expect(result.tool).toBeDefined();
    expect(result.tool!.calls).toBe(2);
  });

  it("tools list is sorted by call count descending", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    for (let i = 1; i < result.tools.length; i++) {
      expect(result.tools[i - 1].calls).toBeGreaterThanOrEqual(result.tools[i].calls);
    }
  });
});
