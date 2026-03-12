import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { Database } from "bun:sqlite";
import { mkdirSync, writeFileSync, existsSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { createTables } from "./schema.js";
import { drainJsonl } from "./drain.js";

const TMP = join(tmpdir(), `sensei-drain-test-${Date.now()}`);
const JSONL_PATH = join(TMP, "events.jsonl");

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

function makeEvent(overrides: Record<string, unknown> = {}) {
  return JSON.stringify({
    user_uuid: "u1",
    session_id: "s1",
    ts: Date.now(),
    tool: "Bash",
    phase: "pre",
    project_path: "/proj",
    ...overrides,
  });
}

describe("drainJsonl", () => {
  it("imports events from JSONL into SQLite and deletes the file", async () => {
    const db = new Database(":memory:");
    createTables(db);

    writeFileSync(JSONL_PATH, [makeEvent(), makeEvent({ tool: "Read" })].join("\n") + "\n");

    await drainJsonl(db, JSONL_PATH);

    const rows = db.prepare("SELECT tool FROM events ORDER BY id").all() as Array<{ tool: string }>;
    expect(rows).toHaveLength(2);
    expect(rows[0].tool).toBe("Bash");
    expect(rows[1].tool).toBe("Read");
    expect(existsSync(JSONL_PATH)).toBe(false);
  });

  it("is a no-op when the JSONL file does not exist", async () => {
    const db = new Database(":memory:");
    createTables(db);

    await expect(drainJsonl(db, JSONL_PATH)).resolves.not.toThrow();

    const count = (db.prepare("SELECT COUNT(*) as n FROM events").get() as { n: number }).n;
    expect(count).toBe(0);
  });

  it("skips malformed JSON lines and still deletes the file", async () => {
    const db = new Database(":memory:");
    createTables(db);

    // JSON parse failures are benign — skipped, valid lines still imported
    const lines = [
      makeEvent({ tool: "Read" }),
      "not valid json",
      makeEvent({ tool: "Glob" }),
    ].join("\n") + "\n";
    writeFileSync(JSONL_PATH, lines);

    await drainJsonl(db, JSONL_PATH);

    // File is deleted because the parse errors are non-fatal (not SQLite errors)
    expect(existsSync(JSONL_PATH)).toBe(false);

    const rows = db.prepare("SELECT tool FROM events ORDER BY id").all() as Array<{ tool: string }>;
    expect(rows).toHaveLength(2);
    expect(rows.map(r => r.tool)).toEqual(["Read", "Glob"]);
  });

  it("leaves JSONL intact if a SQLite transaction fails", async () => {
    // Simulate a SQLite failure by passing a db that has no events table
    const brokenDb = new Database(":memory:");
    // intentionally do NOT call createTables — events table does not exist

    writeFileSync(JSONL_PATH, makeEvent() + "\n");

    await drainJsonl(brokenDb, JSONL_PATH);

    // File must remain intact because the SQLite insert failed
    expect(existsSync(JSONL_PATH)).toBe(true);
  });

  it("skips blank lines silently", async () => {
    const db = new Database(":memory:");
    createTables(db);

    writeFileSync(JSONL_PATH, `\n${makeEvent()}\n\n${makeEvent({ tool: "Write" })}\n`);

    await drainJsonl(db, JSONL_PATH);

    const count = (db.prepare("SELECT COUNT(*) as n FROM events").get() as { n: number }).n;
    expect(count).toBe(2);
  });
});
