import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock @xenova/transformers before importing embedder
vi.mock("@xenova/transformers", () => ({
  pipeline: vi.fn().mockResolvedValue(
    vi.fn().mockResolvedValue({
      data: new Float32Array(384).fill(0.1),
    })
  ),
  env: { cacheDir: "/tmp/xenova-test-cache", localModelPath: "" },
}));

import { embed, isAvailable, ensureReady } from "./embedder.js";

describe("embed", () => {
  it("returns array of length 384", async () => {
    const result = await embed("hello world");
    expect(result).toHaveLength(384);
    expect(typeof result[0]).toBe("number");
  });

  it("same text input produces same vector (deterministic)", async () => {
    const a = await embed("authenticate user");
    const b = await embed("authenticate user");
    expect(a).toEqual(b);
  });

  it("reuses pipeline singleton — pipeline() called only once across multiple embed() calls", async () => {
    const { pipeline } = await import("@xenova/transformers");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const pipelineMock = pipeline as any;
    pipelineMock.mockClear();
    await embed("first call");
    await embed("second call");
    // pipeline() should have been called at most once (lazy singleton)
    expect(pipelineMock.mock.calls.length).toBeLessThanOrEqual(1);
  });
});

describe("isAvailable", () => {
  it("returns false when cache directory absent", async () => {
    // The mock env.cacheDir points to /tmp/xenova-test-cache which doesn't exist
    const result = await isAvailable();
    expect(result).toBe(false);
  });
});

describe("ensureReady", () => {
  it("resolves without throwing", async () => {
    await expect(ensureReady()).resolves.toBeUndefined();
  });
});
