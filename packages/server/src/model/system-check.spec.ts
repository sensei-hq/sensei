import { describe, it, expect } from "vitest";
import { checkSystemRequirements, getDiskFreeGB, getRamGB } from "./system-check.js";

describe("checkSystemRequirements", () => {
  it("returns ollamaBinary:false when execFn throws", async () => {
    const status = await checkSystemRequirements({
      execFn: () => { throw new Error("not found"); },
      fetchFn: async () => { throw new Error("unreachable"); },
    });
    expect(status.ollamaBinary).toBe(false);
    expect(status.ollamaRunning).toBe(false);
    expect(status.ollamaModel).toBe(false);
  });

  it("returns ollamaRunning:false when service is down", async () => {
    const status = await checkSystemRequirements({
      execFn: () => "/usr/local/bin/ollama",
      fetchFn: async () => { throw new Error("ECONNREFUSED"); },
    });
    expect(status.ollamaBinary).toBe(true);
    expect(status.ollamaRunning).toBe(false);
    expect(status.ollamaModel).toBe(false);
  });

  it("returns ollamaModel:true when model is in tags list", async () => {
    const status = await checkSystemRequirements({
      execFn: () => "/usr/local/bin/ollama",
      fetchFn: async (url: string) => {
        if (url.includes("/api/tags")) {
          return new Response(
            JSON.stringify({ models: [{ name: "llama3.2:3b" }] }),
            { status: 200 }
          );
        }
        throw new Error("unexpected url: " + url);
      },
    });
    expect(status.ollamaBinary).toBe(true);
    expect(status.ollamaRunning).toBe(true);
    expect(status.ollamaModel).toBe(true);
    expect(status.ollamaModelName).toBe("llama3.2:3b");
  });

  it("returns ollamaModel:false when model list is empty", async () => {
    const status = await checkSystemRequirements({
      execFn: () => "/usr/local/bin/ollama",
      fetchFn: async () => new Response(JSON.stringify({ models: [] }), { status: 200 }),
    });
    expect(status.ollamaRunning).toBe(true);
    expect(status.ollamaModel).toBe(false);
  });

  it("includes diskFreeGB and ramTotalGB as numbers", async () => {
    const status = await checkSystemRequirements({
      execFn: () => "0",
      fetchFn: async () => { throw new Error(); },
    });
    expect(typeof status.diskFreeGB).toBe("number");
    expect(typeof status.ramTotalGB).toBe("number");
    expect(typeof status.ramAvailableGB).toBe("number");
  });
});

describe("getDiskFreeGB", () => {
  it("returns a number", () => {
    const result = getDiskFreeGB(() => "Filesystem 1024-blocks Used Available Capacity\n/dev/disk1 100000000 50000000 40000000 56%");
    expect(typeof result).toBe("number");
  });

  it("returns 0 on exec failure", () => {
    expect(getDiskFreeGB(() => { throw new Error(); })).toBe(0);
  });
});

describe("getRamGB", () => {
  it("returns 0s on unsupported platform or exec failure", () => {
    const result = getRamGB(() => { throw new Error(); });
    expect(result.total).toBe(0);
    expect(result.available).toBe(0);
  });
});
