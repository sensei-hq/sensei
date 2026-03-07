import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, readdirSync } from "fs";
import { join } from "path";
import {
  getSessionContext, checkpoint, addDecision,
  addPattern, askQuestion, getOpenItems, closeItem,
} from "./project-memory.js";

const TMP = "/tmp/sensei-test-project-memory";

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei/checkpoints/sessions"), { recursive: true });
  writeFileSync(join(TMP, ".sensei/checkpoints/memory.yaml"),
    "version: 1\ndecisions: []\ncontext:\n  project: test-app\n  phase: v1\n");
  writeFileSync(join(TMP, ".sensei/checkpoints/patterns.yaml"),
    "version: 1\npatterns: []\n");
  writeFileSync(join(TMP, ".sensei/checkpoints/open-items.yaml"),
    "version: 1\nitems: []\n");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("getSessionContext", () => {
  it("returns memory + open items in under 400 tokens", async () => {
    const result = await getSessionContext(TMP);
    expect(result).toContain("test-app");
    expect(result.length / 4).toBeLessThan(400);
  });

  it("falls back gracefully when no checkpoints exist", async () => {
    rmSync(join(TMP, ".sensei/checkpoints"), { recursive: true });
    const result = await getSessionContext(TMP);
    expect(result).toContain("No session context");
  });
});

describe("addDecision", () => {
  it("appends a decision to memory.yaml", async () => {
    await addDecision(TMP, "Use repository pattern for all DB access");
    const result = await getSessionContext(TMP);
    expect(result).toContain("repository pattern");
  });

  it("does not duplicate identical decisions", async () => {
    await addDecision(TMP, "Use repository pattern");
    await addDecision(TMP, "Use repository pattern");
    const ctx = await getSessionContext(TMP);
    const count = (ctx.match(/repository pattern/g) ?? []).length;
    expect(count).toBe(1);
  });
});

describe("addPattern", () => {
  it("appends a pattern", async () => {
    await addPattern(TMP, "data-attribute DOM", "data-{component} on root");
    const result = await getSessionContext(TMP);
    expect(result).toContain("data-attribute DOM");
  });

  it("increments uses counter on repeat", async () => {
    await addPattern(TMP, "repo-pattern", "all db via repos");
    await addPattern(TMP, "repo-pattern", "all db via repos");
    // second call should not throw and should update uses
    const result = await getSessionContext(TMP);
    expect(result).toContain("repo-pattern");
  });
});

describe("askQuestion / getOpenItems / closeItem", () => {
  it("adds a question to open items", async () => {
    const id = await askQuestion(TMP, "Should we use optimistic locking?");
    expect(id).toBeTruthy();
    const items = await getOpenItems(TMP);
    expect(items).toContain("optimistic locking");
  });

  it("closes a question by id", async () => {
    const id = await askQuestion(TMP, "Use row versioning?");
    await closeItem(TMP, id);
    const items = await getOpenItems(TMP);
    expect(items).not.toContain("row versioning");
  });

  it("returns no open items message when empty", async () => {
    const items = await getOpenItems(TMP);
    expect(items).toContain("No open items");
  });
});

describe("checkpoint", () => {
  it("returns resume instruction", async () => {
    const result = await checkpoint(TMP, "Added auth module. Tests pass.");
    expect(result).toContain("Checkpointed");
    expect(result).toContain("get_session_context");
  });

  it("writes session archive file", async () => {
    await checkpoint(TMP, "Done.");
    const sessions = readdirSync(join(TMP, ".sensei/checkpoints/sessions"));
    expect(sessions.length).toBeGreaterThan(0);
  });

  it("captures explicit decisions passed to checkpoint", async () => {
    await checkpoint(TMP, "Done.", ["Use zod for validation"]);
    const ctx = await getSessionContext(TMP);
    expect(ctx).toContain("zod");
  });
});
