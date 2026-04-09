import { readFile, writeFile, mkdir } from "fs/promises";
import { join, dirname } from "path";
import { homedir } from "os";
import { existsSync } from "fs";
import { createRequire } from "module";
import { intro, outro, log, spinner } from "@clack/prompts";
import { extractProjectProfile, SkillGenerator, SkillValidator, ClaudeAdapter } from "@sensei/engine";
import { ClaudeBackend } from "@sensei/server";
import { loadSenseiConfig, type AgentSkillsManifest } from "@sensei/shared";

const MCP_CONFIG = join(homedir(), ".claude", "mcp.json");
const SETTINGS_PATH = join(homedir(), ".claude", "settings.json");

/** Resolve path to the instrumented MCP entry point in @sensei/server */
export function resolveMcpEntryPath(): string {
  const _require = createRequire(import.meta.url);
  const serverPkgPath = _require.resolve("@sensei/server/package.json");
  return join(dirname(serverPkgPath), "src", "mcp-entry.ts");
}

export async function setupMcp(repoPath: string, mcpEntryPath?: string): Promise<void> {
  intro("sensei setup --mcp");

  const entryPath = mcpEntryPath ?? resolveMcpEntryPath();

  await mkdir(dirname(MCP_CONFIG), { recursive: true });

  // 1. Register MCP server
  let mcpConfig: { mcpServers?: Record<string, unknown> } = { mcpServers: {} };
  if (existsSync(MCP_CONFIG)) {
    try { mcpConfig = JSON.parse(await readFile(MCP_CONFIG, "utf-8")); } catch {}
  }
  mcpConfig.mcpServers ??= {};
  mcpConfig.mcpServers["sensei"] = {
    command: "bun",
    args: [entryPath],
    env: { SENSEI_REPO_PATH: repoPath },
  };
  await writeFile(MCP_CONFIG, JSON.stringify(mcpConfig, null, 2), "utf-8");
  log.success(`MCP server registered in ${MCP_CONFIG}`);
  log.info(`command: bun ${entryPath}`);
  log.info(`SENSEI_REPO_PATH: ${repoPath}`);

  // 2. Write OTEL_EXPORTER_OTLP_ENDPOINT into ~/.claude/settings.json so Claude Code
  //    sends telemetry to the collector daemon OTLP endpoint.
  let settings: Record<string, unknown> = {};
  if (existsSync(SETTINGS_PATH)) {
    try { settings = JSON.parse(await readFile(SETTINGS_PATH, "utf-8")); } catch {}
  }
  const env = (settings.env as Record<string, string>) ?? {};
  env["OTEL_EXPORTER_OTLP_ENDPOINT"] = "http://localhost:51789";
  settings.env = env;
  await writeFile(SETTINGS_PATH, JSON.stringify(settings, null, 2), "utf-8");
  log.success(`OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:51789 written to ${SETTINGS_PATH}`);

  outro("Done. Restart Claude Code to pick up the MCP server.");
}

export async function setupHooks(): Promise<void> {
  intro("sensei setup --hooks");
  const { installHooks } = await import("@sensei/collector");
  await installHooks();
  log.success("Hook scripts installed to ~/.claude/hooks/");
  log.success("hooks.PreToolUse and hooks.PostToolUse registered in ~/.claude/settings.json");
  log.info("Daemon autostart registered via launchd (macOS only — Linux systemd not yet supported)");
  log.info("Run: launchctl load ~/Library/LaunchAgents/com.sensei.collector.plist");
  outro("Done. Claude tool calls will now be tracked in ~/.sensei/<uuid>/analytics.db");
}

export async function setupAgent(repoPath: string, agent: string): Promise<void> {
  if (agent !== "claude") {
    console.error(`Agent '${agent}' is not yet supported. Supported: claude`);
    process.exit(1);
  }

  intro(`sensei setup --agent ${agent}`);

  // 1. Load config
  const config = await loadSenseiConfig(repoPath);
  if (!config?.repo_id) throw new Error("Repo not configured. Run sensei init first.");

  // 2. Extract project profile
  const s1 = spinner();
  s1.start("Analysing project...");
  const profile = await extractProjectProfile(config.repo_id, repoPath);
  s1.stop(`Project analysed: ${profile.dominantLanguage}, ${profile.packageNames.length} packages`);

  // 3. Init Claude backend — fails fast if ANTHROPIC_API_KEY missing
  const backend = new ClaudeBackend();
  await backend.init();

  // 4. Generate + validate all 4 skills
  const validator = new SkillValidator(backend, profile);
  const generator = new SkillGenerator(backend, profile, validator);

  const s2 = spinner();
  s2.start("Generating skills...");
  const skills = await generator.generate();
  s2.stop("Skills generated (4/4)");

  // 5. Write skill files to ~/.claude/skills/
  const adapter = new ClaudeAdapter();
  const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const written = await adapter.writeSkills(skills, repoSlug);

  // 6. Write manifest to .sensei/agent-skills.json
  const manifest: AgentSkillsManifest = {
    agent: "claude",
    repoSlug,
    skills: written,
    updatedAt: new Date().toISOString(),
  };
  const senseiDir = join(repoPath, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  await writeFile(join(senseiDir, "agent-skills.json"), JSON.stringify(manifest, null, 2), "utf-8");

  log.success(`4 skill files written to ${adapter.skillsDir}`);
  written.forEach(f => log.info(`  ${f.path}`));
  outro("Done. Restart Claude Code to pick up the new skills.");
}
