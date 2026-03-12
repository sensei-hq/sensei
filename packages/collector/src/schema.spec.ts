import { describe, it, expect, afterEach } from "vitest";
import { Database } from "bun:sqlite";
import { mkdirSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { createTables } from "./schema.js";

const TMP = join(tmpdir(), `sensei-schema-test-${Date.now()}`);

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("createTables", () => {
  it("creates events table with correct columns including seq", () => {
    const db = new Database(":memory:");
    createTables(db);

    const cols = db
      .prepare("PRAGMA table_info(events)")
      .all() as Array<{ name: string; type: string; notnull: number }>;

    const names = cols.map(c => c.name);
    expect(names).toContain("id");
    expect(names).toContain("user_uuid");
    expect(names).toContain("session_id");
    expect(names).toContain("seq");
    expect(names).toContain("ts");
    expect(names).toContain("tool");
    expect(names).toContain("phase");
    expect(names).toContain("duration_ms");
    expect(names).toContain("success");
    expect(names).toContain("input");
    expect(names).toContain("error");
    expect(names).toContain("project_path");
  });

  it("creates projects table", () => {
    const db = new Database(":memory:");
    createTables(db);
    const cols = db.prepare("PRAGMA table_info(projects)").all() as Array<{ name: string }>;
    const names = cols.map(c => c.name);
    expect(names).toContain("path");
    expect(names).toContain("first_seen");
    expect(names).toContain("last_seen");
  });

  it("creates daily_stats table", () => {
    const db = new Database(":memory:");
    createTables(db);
    const cols = db.prepare("PRAGMA table_info(daily_stats)").all() as Array<{ name: string }>;
    const names = cols.map(c => c.name);
    expect(names).toContain("date");
    expect(names).toContain("tool");
    expect(names).toContain("calls");
    expect(names).toContain("successes");
    expect(names).toContain("total_duration_ms");
  });

  it("is idempotent — runs twice without error", () => {
    const db = new Database(":memory:");
    createTables(db);
    expect(() => createTables(db)).not.toThrow();
  });

  it("enables WAL mode on a real file database", () => {
    mkdirSync(TMP, { recursive: true });
    const db = new Database(join(TMP, "test.db"));
    createTables(db);
    const row = db.prepare("PRAGMA journal_mode").get() as { journal_mode: string };
    expect(row.journal_mode).toBe("wal");
    db.close();
  });
});
