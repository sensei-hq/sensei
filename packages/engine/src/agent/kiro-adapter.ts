import { homedir } from "os";
import { join } from "path";
import { existsSync } from "fs";
import { BaseAcpAdapter } from "./base-adapter.js";
import { readJsonConfig, writeJsonConfig, findSenseiBinary } from "./acp-utils.js";

export class KiroAdapter extends BaseAcpAdapter {
  readonly id = "kiro";
  readonly name = "Kiro";
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    super();
    this.skillsDir = skillsDir ?? join(homedir(), ".kiro", "skills");
  }

  async detect(): Promise<boolean> {
    return existsSync(join(homedir(), ".kiro"));
  }

  async registerMcp(senseiCmd?: string): Promise<void> {
    const cmd = senseiCmd ?? findSenseiBinary();
    const mcpPath = join(homedir(), ".kiro", "settings", "mcp.json");
    const config = await readJsonConfig(mcpPath);
    const mcpServers = (config.mcpServers as Record<string, unknown>) ?? {};
    mcpServers["sensei"] = { command: cmd, args: ["mcp"] };
    config.mcpServers = mcpServers;
    await writeJsonConfig(mcpPath, config);
  }
}
