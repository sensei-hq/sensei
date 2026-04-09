// packages/cli/src/commands/watch.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@sensei/graph-indexer", () => ({
  watchRepo: vi.fn(),
}));

vi.mock("@sensei/shared", () => ({
  loadSenseiConfig: vi.fn(),
}));

import { watch } from "./watch.js";
import { watchRepo } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";

const mockWatchRepo = watchRepo as ReturnType<typeof vi.fn>;
const mockLoadConfig = loadSenseiConfig as ReturnType<typeof vi.fn>;
const mockStop = vi.fn().mockResolvedValue(undefined);

describe("watch command", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStop.mockResolvedValue(undefined);
    mockWatchRepo.mockResolvedValue({ stop: mockStop, rescan: vi.fn() });
    mockLoadConfig.mockResolvedValue({ repo_id: "test-repo-id" });
  });

  it("exits with error when no config found", async () => {
    mockLoadConfig.mockResolvedValue(null);
    const exitSpy = vi.spyOn(process, "exit").mockImplementation((() => {}) as never);
    await watch("/repo");
    expect(exitSpy).toHaveBeenCalledWith(1);
    exitSpy.mockRestore();
  });

  it("calls watchRepo with correct repoPath and repoId", async () => {
    const watchPromise = watch("/repo");
    // Drain microtask queue so watch() reaches process.once registration
    await Promise.resolve();
    await Promise.resolve();
    process.emit("SIGINT");
    await watchPromise;

    expect(mockWatchRepo).toHaveBeenCalledWith(
      expect.objectContaining({ repoPath: "/repo", repoId: "test-repo-id" })
    );
  });

  it("stops the watcher on SIGINT", async () => {
    const watchPromise = watch("/repo");
    await Promise.resolve();
    await Promise.resolve();
    process.emit("SIGINT");
    await watchPromise;
    expect(mockStop).toHaveBeenCalled();
  });
});
