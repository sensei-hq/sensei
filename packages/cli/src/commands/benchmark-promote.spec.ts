import { describe, it, expect, vi, beforeEach } from "vitest";
import { buildFeedback, submitReport } from "./benchmark-promote.js";

describe("buildFeedback", () => {
  it("sets systemAgreed true when user matches auto", () => {
    const fb = buildFeedback("c", "c", "great output");
    expect(fb).toEqual({ preferred: "c", systemAgreed: true, note: "great output" });
  });

  it("sets systemAgreed false when user overrides", () => {
    const fb = buildFeedback("b", "c");
    expect(fb).toEqual({ preferred: "b", systemAgreed: false });
  });
});

describe("submitReport", () => {
  beforeEach(() => {
    global.fetch = vi.fn().mockResolvedValue({ json: async () => ({ ok: true }) });
  });

  it("POSTs to telemetry URL", async () => {
    await submitReport({ id: "test-id" }, "http://localhost:7744");
    expect(global.fetch).toHaveBeenCalledWith(
      "http://localhost:7744/reports",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("does not throw when server is down", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("ECONNREFUSED"));
    await expect(submitReport({ id: "test-id" }, "http://localhost:7744")).resolves.toBeUndefined();
  });
});
