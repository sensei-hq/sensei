import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { Database } from "bun:sqlite";
import { mkdirSync, rmSync, writeFileSync, existsSync } from "fs";
import { join, dirname } from "path";
import { tmpdir } from "os";
import { createTables, queryStats } from "@sensei/collector";
import { formatStats, stats } from "./stats.js";

const TMP = join(tmpdir(), `sensei-stats-cmd-test-${Date.now()}`);

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

function seedDb(db: Database) {
  const insert = db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);
  const recent = Date.now() - 1000;
  insert.run("u1", "s1", 1, recent, "search_index", "post", 150, 1, null, null, "/proj");
  insert.run("u1", "s1", 2, recent + 100, "Bash", "post", 300, 0, null, "error", "/proj");
}

describe("formatStats", () => {
  it("default text output includes tool names and total calls", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    const text = formatStats(result, { json: false });
    expect(text).toContain("search_index");
    expect(text).toContain("Bash");
    expect(text).toContain("2"); // total calls
  });

  it("--json output is valid JSON with expected keys", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    const text = formatStats(result, { json: true });
    const parsed = JSON.parse(text) as Record<string, unknown>;
    expect(parsed.total_calls).toBeDefined();
    expect(Array.isArray(parsed.tools)).toBe(true);
    expect(parsed.period).toBeDefined();
  });

  it("tool-specific output includes tool name and success_rate", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { tool: "search_index" });
    const text = formatStats(result, { json: false });
    expect(text).toContain("search_index");
    expect(text).toMatch(/100%|1\.0/);
  });
});

describe("stats() DB path construction", () => {
  it("opens analytics.db at ~/.sensei/<uuid>/analytics.db", async () => {
    const home = join(TMP, "home");
    const uuidVal = "test-uuid-abc";
    mkdirSync(join(home, ".sensei"), { recursive: true });
    writeFileSync(join(home, ".sensei", "uuid"), uuidVal);

    // Capture console.log output
    const output: string[] = [];
    const origLog = console.log;
    console.log = (...args: unknown[]) => output.push(args.join(" "));
    try {
      await stats({ _home: home });
    } finally {
      console.log = origLog;
    }

    // The DB file must have been created at the expected path
    const dbPath = join(home, ".sensei", uuidVal, "analytics.db");
    expect(existsSync(dbPath)).toBe(true);
  });
});
