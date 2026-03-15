import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

describe("ClaudeBackend", () => {
  const originalKey = process.env.ANTHROPIC_API_KEY;

  afterEach(() => {
    if (originalKey === undefined) delete process.env.ANTHROPIC_API_KEY;
    else process.env.ANTHROPIC_API_KEY = originalKey;
  });

  it("init() throws when ANTHROPIC_API_KEY is absent", async () => {
    delete process.env.ANTHROPIC_API_KEY;
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    await expect(backend.init()).rejects.toThrow("ANTHROPIC_API_KEY");
  });

  it("isAvailable() returns false when key absent", async () => {
    delete process.env.ANTHROPIC_API_KEY;
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    expect(await backend.isAvailable()).toBe(false);
  });

  it("isAvailable() returns true when key present", async () => {
    process.env.ANTHROPIC_API_KEY = "test-key";
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    expect(await backend.isAvailable()).toBe(true);
  });

  it("embed() throws NotImplementedError", async () => {
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    await expect(backend.embed("text")).rejects.toThrow("does not support embed");
  });

  it("extract() throws NotImplementedError", async () => {
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    await expect(backend.extract("content", { filePath: "foo.ts" })).rejects.toThrow("does not support extract");
  });

  it("generate() before init() throws", async () => {
    process.env.ANTHROPIC_API_KEY = "test-key";
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    // client is null until init() — should throw
    await expect(backend.generate("hello")).rejects.toThrow("not initialized");
  });
});
