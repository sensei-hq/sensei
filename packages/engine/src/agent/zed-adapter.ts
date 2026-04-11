import { homedir } from "os";
import { join } from "path";
import { existsSync } from "fs";
import { BaseAcpAdapter } from "./base-adapter.js";
import { readJsonConfig, writeJsonConfig, findSenseiBinary } from "./acp-utils.js";

export class ZedAdapter extends BaseAcpAdapter {
  readonly id = "zed";
  readonly name = "Zed";
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    super();
    this.skillsDir = skillsDir ?? join(homedir(), ".config", "zed", "sensei-skills");
  }

  async detect(): Promise<boolean> {
    return existsSync(join(homedir(), ".config", "zed"));
  }

  /**
   * Register MCP server in ~/.config/zed/settings.json → context_servers.sensei.
   * Zed uses a different schema: { command: { path, args } }.
   */
  async registerMcp(senseiCmd?: string): Promise<void> {
    const cmd = senseiCmd ?? findSenseiBinary();
    const settingsPath = join(homedir(), ".config", "zed", "settings.json");
    const settings = await readJsonConfig(settingsPath);
    const contextServers = (settings.context_servers as Record<string, unknown>) ?? {};
    contextServers["sensei"] = { command: { path: cmd, args: ["mcp"] } };
    settings.context_servers = contextServers;
    await writeJsonConfig(settingsPath, settings);
  }
}
