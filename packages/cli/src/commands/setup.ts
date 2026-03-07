import { readFile, writeFile, mkdir } from "fs/promises";
import { join, dirname } from "path";
import { homedir } from "os";
import { existsSync } from "fs";
import { intro, outro, log } from "@clack/prompts";

const MCP_CONFIG = join(homedir(), ".claude", "mcp.json");

export async function setupMcp(repoPath: string, indexJsPath: string): Promise<void> {
  intro("sensei setup --mcp");

  await mkdir(dirname(MCP_CONFIG), { recursive: true });

  let config: { mcpServers?: Record<string, unknown> } = { mcpServers: {} };
  if (existsSync(MCP_CONFIG)) {
    try {
      config = JSON.parse(await readFile(MCP_CONFIG, "utf-8"));
    } catch {
      // start fresh
    }
  }

  config.mcpServers ??= {};
  config.mcpServers["sensei"] = {
    command: "bun",
    args: [indexJsPath],
    env: { REPO_PATH: repoPath },
  };

  await writeFile(MCP_CONFIG, JSON.stringify(config, null, 2), "utf-8");
  log.success(`MCP server registered in ${MCP_CONFIG}`);
  log.info(`command: bun ${indexJsPath}`);
  log.info(`REPO_PATH: ${repoPath}`);
  outro("Done. Restart Claude Code to pick up the MCP server.");
}
