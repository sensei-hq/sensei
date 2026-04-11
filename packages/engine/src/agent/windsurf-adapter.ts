import { homedir } from "os";
import { join } from "path";
import { existsSync } from "fs";
import { writeFile } from "fs/promises";
import { BaseAcpAdapter } from "./base-adapter.js";
import { readJsonConfig, writeJsonConfig, findSenseiBinary } from "./acp-utils.js";

export class WindsurfAdapter extends BaseAcpAdapter {
  readonly id = "windsurf";
  readonly name = "Windsurf";
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    super();
    this.skillsDir = skillsDir ?? join(homedir(), ".codeium", "windsurf", "memories");
  }

  async detect(): Promise<boolean> {
    return existsSync(join(homedir(), ".codeium", "windsurf"));
  }

  async registerMcp(senseiCmd?: string): Promise<void> {
    const cmd = senseiCmd ?? findSenseiBinary();
    const mcpPath = join(homedir(), ".codeium", "windsurf", "mcp.json");
    const config = await readJsonConfig(mcpPath);
    const mcpServers = (config.mcpServers as Record<string, unknown>) ?? {};
    mcpServers["sensei"] = { command: cmd, args: ["mcp"] };
    config.mcpServers = mcpServers;
    await writeJsonConfig(mcpPath, config);
  }

  async writeRules(content: string, repoPath: string): Promise<string> {
    const rulesPath = join(repoPath, ".windsurfrules");
    await writeFile(rulesPath, content, "utf-8");
    return rulesPath;
  }
}
