import { homedir } from "os";
import { join } from "path";
import { existsSync } from "fs";
import { writeFile } from "fs/promises";
import { BaseAcpAdapter } from "./base-adapter.js";
import { readJsonConfig, writeJsonConfig, findSenseiBinary } from "./acp-utils.js";

export class CursorAdapter extends BaseAcpAdapter {
  readonly id = "cursor";
  readonly name = "Cursor";
  readonly skillsDir: string;
  protected readonly skillExt = ".mdc";

  constructor(skillsDir?: string) {
    super();
    this.skillsDir = skillsDir ?? join(homedir(), ".cursor", "rules");
  }

  async detect(): Promise<boolean> {
    return existsSync(join(homedir(), ".cursor"));
  }

  async registerMcp(senseiCmd?: string): Promise<void> {
    const cmd = senseiCmd ?? findSenseiBinary();
    const mcpPath = join(homedir(), ".cursor", "mcp.json");
    const config = await readJsonConfig(mcpPath);
    const mcpServers = (config.mcpServers as Record<string, unknown>) ?? {};
    mcpServers["sensei"] = { command: cmd, args: ["mcp"] };
    config.mcpServers = mcpServers;
    await writeJsonConfig(mcpPath, config);
  }

  async writeRules(content: string, repoPath: string): Promise<string> {
    const rulesPath = join(repoPath, ".cursorrules");
    await writeFile(rulesPath, content, "utf-8");
    return rulesPath;
  }
}
