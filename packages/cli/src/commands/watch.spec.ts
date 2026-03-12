// packages/cli/src/commands/watch.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@sensei/tools", () => ({
  reindexRepo: vi.fn(),
  checkDrift: vi.fn(),
}));

vi.mock("chokidar", () => ({
  default: {
    watch: vi.fn(),
  },
}));

vi.mock("fs", () => ({
  existsSync: vi.fn(() => true),
}));

import { watch } from "./watch.js";
import { reindexRepo, checkDrift } from "@sensei/tools";
import chokidar from "chokidar";

const mockReindex = reindexRepo as ReturnType<typeof vi.fn>;
const mockCheckDrift = checkDrift as ReturnType<typeof vi.fn>;
const mockChokidar = chokidar.watch as ReturnType<typeof vi.fn>;

function makeWatcher() {
  const handlers: Record<string, (() => void)[]> = {};
  const watcher = {
    on: vi.fn().mockImplementation((event: string, handler: () => void) => {
      handlers[event] = handlers[event] ?? [];
      handlers[event].push(handler);
      return watcher;
    }),
    close: vi.fn().mockResolvedValue(undefined),
    emit: (event: string) => handlers[event]?.forEach(h => h()),
  };
  return watcher;
}

describe("watch --drift", () => {
  let watcher: ReturnType<typeof makeWatcher>;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    watcher = makeWatcher();
    mockChokidar.mockReturnValue(watcher);
    mockReindex.mockResolvedValue({ added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false });
    mockCheckDrift.mockResolvedValue({ drifted: [], summary: "No drift detected.", lastIndexedCommit: "abc1234" });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  async function triggerReindexAndStop() {
    // Fire a file change event to trigger debounce
    watcher.emit("change");
    // Advance past the debounce timeout (500ms) and drain timers + microtask queue
    vi.runAllTimers();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  }

  it("does NOT call checkDrift when drift option is false", async () => {
    const watchPromise = watch("/repo", { drift: false });
    await triggerReindexAndStop();
    process.emit("SIGINT");
    await watchPromise;
    expect(mockCheckDrift).not.toHaveBeenCalled();
  });

  it("calls checkDrift after reindex when drift option is true", async () => {
    const watchPromise = watch("/repo", { drift: true });
    await triggerReindexAndStop();
    process.emit("SIGINT");
    await watchPromise;
    expect(mockCheckDrift).toHaveBeenCalledWith("/repo");
  });

  it("prints drift summary when drift found", async () => {
    const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    mockCheckDrift.mockResolvedValue({
      drifted: [{ docPath: "docs/design/03-mcp-server.md", reason: "code-changed", changedFiles: ["packages/mcp/src/index.ts"] }],
      summary: "1 doc(s) drifted since abc1234:\ndocs/design/03-mcp-server.md: code changed — packages/mcp/src/index.ts",
    });

    const watchPromise = watch("/repo", { drift: true });
    await triggerReindexAndStop();
    process.emit("SIGINT");
    await watchPromise;

    expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining("1 doc(s) drifted"));
    consoleSpy.mockRestore();
  });

  it("is silent when no drift found", async () => {
    const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});

    const watchPromise = watch("/repo", { drift: true });
    await triggerReindexAndStop();
    process.emit("SIGINT");
    await watchPromise;

    const driftLogs = consoleSpy.mock.calls.filter(c => String(c[0]).includes("drift"));
    expect(driftLogs).toHaveLength(0);
    consoleSpy.mockRestore();
  });

  it("continues watching if checkDrift throws", async () => {
    const consoleWarnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    mockCheckDrift.mockRejectedValue(new Error("drift check failed"));

    const watchPromise = watch("/repo", { drift: true });
    await triggerReindexAndStop();
    process.emit("SIGINT");
    await expect(watchPromise).resolves.toBeUndefined();
    expect(consoleWarnSpy).toHaveBeenCalledWith("drift check error:", "drift check failed");
    consoleWarnSpy.mockRestore();
  });
});
