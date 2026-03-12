import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, rmSync, existsSync, readFileSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { readOrCreateUuid } from "./uuid.js";

const TMP = join(tmpdir(), `sensei-uuid-test-${Date.now()}`);

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("readOrCreateUuid", () => {
  it("creates a UUID file on first call", async () => {
    const uuidPath = join(TMP, "uuid");
    const id = await readOrCreateUuid(uuidPath);
    expect(id).toMatch(/^[0-9a-f-]{36}$/);
    expect(existsSync(uuidPath)).toBe(true);
    expect(readFileSync(uuidPath, "utf8").trim()).toBe(id);
  });

  it("returns the same UUID on repeated calls", async () => {
    const uuidPath = join(TMP, "uuid");
    const a = await readOrCreateUuid(uuidPath);
    const b = await readOrCreateUuid(uuidPath);
    expect(a).toBe(b);
  });

  it("does not overwrite an existing UUID", async () => {
    const uuidPath = join(TMP, "uuid");
    // Pre-seed with a known UUID
    const { writeFileSync } = await import("fs");
    writeFileSync(uuidPath, "preset-uuid-value");
    const id = await readOrCreateUuid(uuidPath);
    expect(id).toBe("preset-uuid-value");
  });
});
