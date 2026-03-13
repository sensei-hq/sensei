import { describe, it, expect } from "vitest";
import { createSenseiMcpServer } from "./mcp-server.js";

describe("createSenseiMcpServer", () => {
  it("returns a server object with a connect method", () => {
    const server = createSenseiMcpServer({ repoId: "test", repoPath: "/tmp" });
    expect(server).toBeDefined();
    expect(typeof server.connect).toBe("function");
  });
});
