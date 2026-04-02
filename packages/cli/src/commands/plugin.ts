import { cp, mkdir, readFile, writeFile, readdir } from "node:fs/promises";
import { existsSync, readdirSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";
import { intro, outro, log, spinner } from "@clack/prompts";

const PLUGIN_NAME = "sensei";
const PLUGIN_VERSION = "1.0.0";
const PLUGIN_KEY = `${PLUGIN_NAME}@local`;

/** Find the plugin source directory relative to this CLI script. */
function findPluginSrc(): string | null {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  const cwd = process.cwd();
  const candidates = [
    join(scriptDir, "..", "plugin"),               // packages/cli/plugin/ (production bundle)
    join(scriptDir, "..", "..", "..", "plugin"),    // <repo-root>/plugin/ (dev / monorepo)
    join(cwd, "plugin"),                           // cwd/plugin/ (running from repo root)
  ];
  for (const dir of candidates) {
    const resolved = resolve(dir);
    if (existsSync(join(resolved, ".claude-plugin", "plugin.json"))) {
      return resolved;
    }
  }
  return null;
}

export async function pluginInstall(): Promise<void> {
  intro("sensei plugin install");

  const pluginSrc = findPluginSrc();
  if (!pluginSrc) {
    log.error("Could not locate sensei plugin directory. Run this from inside the sensei repo.");
    process.exit(1);
  }

  const claudeDir = join(homedir(), ".claude");
  const pluginsDir = join(claudeDir, "plugins");
  const installPath = join(pluginsDir, "cache", PLUGIN_NAME, "local", PLUGIN_VERSION);
  const installedPluginsPath = join(pluginsDir, "installed_plugins.json");

  const s = spinner();

  // 1. Copy plugin directory
  s.start(`Copying plugin to ${installPath}`);
  await mkdir(installPath, { recursive: true });
  await cp(pluginSrc, installPath, { recursive: true, force: true });
  s.stop(`Copied plugin files`);

  // 2. Read existing installed_plugins.json
  let registry: { version: number; plugins: Record<string, unknown[]> } = {
    version: 2,
    plugins: {},
  };
  if (existsSync(installedPluginsPath)) {
    const raw = await readFile(installedPluginsPath, "utf-8");
    try {
      registry = JSON.parse(raw);
    } catch {
      log.error("installed_plugins.json is corrupted. Delete it and retry.");
      process.exit(1);
    }
  }

  // 3. Upsert plugin entry
  registry.plugins[PLUGIN_KEY] = [
    {
      scope: "user",
      installPath,
      version: PLUGIN_VERSION,
      installedAt: new Date().toISOString(),
      lastUpdated: new Date().toISOString(),
    },
  ];

  // 4. Write back
  await mkdir(pluginsDir, { recursive: true });
  await writeFile(installedPluginsPath, JSON.stringify(registry, null, 2));
  log.success(`Registered ${PLUGIN_KEY} in installed_plugins.json`);

  // 5. Summary
  const skillsDir = join(installPath, "skills");
  const commandsDir = join(installPath, "commands");
  const skills = existsSync(skillsDir) ? readdirSync(skillsDir, { withFileTypes: true }).filter(e => e.isDirectory()).map(e => e.name) : [];
  const commands = existsSync(commandsDir)
    ? (await readdir(commandsDir)).filter(f => f.endsWith(".md")).map(f => "/" + f.replace(".md", ""))
    : [];

  log.info(`Skills (${skills.length}): ${skills.join(", ")}`);
  log.info(`Commands (${commands.length}): ${commands.join(", ")}`);

  outro("Done. Restart Claude Code to activate the sensei plugin.");
}
