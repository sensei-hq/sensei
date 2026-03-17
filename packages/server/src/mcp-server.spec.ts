import { describe, it, expect } from "vitest";
import { createSenseiMcpServer } from "./mcp-server.js";

describe("createSenseiMcpServer", () => {
  it("returns a server object with a connect method", () => {
    const server = createSenseiMcpServer({ repoId: "test", repoPath: "/tmp" });
    expect(server).toBeDefined();
    expect(typeof server.connect).toBe("function");
  });

  it("exposes record_pattern_use as a registered tool", () => {
    const server = createSenseiMcpServer({ repoId: "test", repoPath: "/tmp" });
    // MCP server stores tools in _registeredTools
    const tools = (server as any)._registeredTools;
    expect(tools).toBeDefined();
    expect(tools["record_pattern_use"]).toBeDefined();
  });
});
