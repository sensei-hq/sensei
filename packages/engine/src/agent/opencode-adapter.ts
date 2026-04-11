import { homedir } from "os";
import { join } from "path";
import { existsSync } from "fs";
import { BaseAcpAdapter } from "./base-adapter.js";
import { readJsonConfig, writeJsonConfig, findSenseiBinary } from "./acp-utils.js";

export class OpenCodeAdapter extends BaseAcpAdapter {
  readonly id = "opencode";
  readonly name = "OpenCode";
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    super();
    this.skillsDir = skillsDir ?? join(homedir(), ".config", "opencode", "sensei-skills");
  }

  async detect(): Promise<boolean> {
    return existsSync(join(homedir(), ".config", "opencode"));
  }

  /**
   * Register MCP server in ~/.config/opencode/opencode.json → mcp.sensei.
   * OpenCode schema: { type: "stdio", command: [cmd, ...args], enabled: true }
   */
  async registerMcp(senseiCmd?: string): Promise<void> {
    const cmd = senseiCmd ?? findSenseiBinary();
    const configPath = join(homedir(), ".config", "opencode", "opencode.json");
    const config = await readJsonConfig(configPath);
    const mcp = (config.mcp as Record<string, unknown>) ?? {};
    mcp["sensei"] = { type: "stdio", command: [cmd, "mcp"], enabled: true };
    config.mcp = mcp;
    await writeJsonConfig(configPath, config);
  }
}
