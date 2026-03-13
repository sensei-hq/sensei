import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, existsSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { drainJsonl } from "./drain.js";

const TMP = join(tmpdir(), `sensei-drain-test-${Date.now()}`);
const JSONL_PATH = join(TMP, "events.jsonl");

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

function makeEvent(overrides: Record<string, unknown> = {}) {
  return JSON.stringify({ user_uuid: "u1", session_id: "s1", ts: Date.now(), tool: "Bash", phase: "pre", project_path: "/proj", ...overrides });
}

function makeMockClient(insertError: Error | null = null) {
  const insertedRows: any[] = [];
  const mockInsert = vi.fn((data: any) => { insertedRows.push(data); return { error: insertError }; });
  const mockFrom = vi.fn(() => ({ insert: mockInsert }));
  return { client: { from: mockFrom } as any, insertedRows, mockInsert };
}

describe("drainJsonl", () => {
  it("drains events to Supabase and deletes file", async () => {
    writeFileSync(JSONL_PATH, [makeEvent(), makeEvent({ tool: "Read" })].join("\n") + "\n");
    const { client, mockInsert } = makeMockClient();
    await drainJsonl(client, JSONL_PATH);
    expect(mockInsert).toHaveBeenCalledTimes(2);
    expect(existsSync(JSONL_PATH)).toBe(false);
  });

  it("is a no-op when the JSONL file does not exist", async () => {
    const { client, mockInsert } = makeMockClient();
    await expect(drainJsonl(client, JSONL_PATH)).resolves.toBeUndefined();
    expect(mockInsert).not.toHaveBeenCalled();
  });

  it("skips malformed JSON lines and still deletes the file", async () => {
    const lines = [makeEvent({ tool: "Read" }), "not valid json", makeEvent({ tool: "Glob" })].join("\n") + "\n";
    writeFileSync(JSONL_PATH, lines);
    const { client, mockInsert } = makeMockClient();
    await drainJsonl(client, JSONL_PATH);
    expect(mockInsert).toHaveBeenCalledTimes(2);
    expect(existsSync(JSONL_PATH)).toBe(false);
  });

  it("skips blank lines silently", async () => {
    writeFileSync(JSONL_PATH, `\n${makeEvent()}\n\n${makeEvent({ tool: "Write" })}\n`);
    const { client, mockInsert } = makeMockClient();
    await drainJsonl(client, JSONL_PATH);
    expect(mockInsert).toHaveBeenCalledTimes(2);
  });
});
