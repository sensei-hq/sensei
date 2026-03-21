import { cp, mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

const PLUGIN_NAME = "sensei";
const PLUGIN_VERSION = "1.0.0";
const PLUGIN_KEY = `${PLUGIN_NAME}@local`;

const __dirname = dirname(fileURLToPath(import.meta.url));
const pluginSrc = resolve(__dirname, "..");
const claudeDir = join(homedir(), ".claude");
const pluginsDir = join(claudeDir, "plugins");
const installPath = join(pluginsDir, "cache", PLUGIN_NAME, "local", PLUGIN_VERSION);
const installedPluginsPath = join(pluginsDir, "installed_plugins.json");

async function install() {
  // 1. Copy plugin directory
  await mkdir(installPath, { recursive: true });
  await cp(pluginSrc, installPath, { recursive: true, force: true });
  console.log(`✓ Copied plugin to ${installPath}`);

  // 2. Read existing installed_plugins.json
  let registry: { version: number; plugins: Record<string, unknown[]> } = {
    version: 2,
    plugins: {},
  };
  if (existsSync(installedPluginsPath)) {
    const raw = await readFile(installedPluginsPath, "utf-8");
    registry = JSON.parse(raw);
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
  await writeFile(installedPluginsPath, JSON.stringify(registry, null, 2));
  console.log(`✓ Registered ${PLUGIN_KEY} in installed_plugins.json`);

  // 5. Summary
  const skillsDir = join(installPath, "skills");
  const commandsDir = join(installPath, "commands");
  const { readdirSync } = await import("node:fs");
  const skills = existsSync(skillsDir) ? readdirSync(skillsDir) : [];
  const commands = existsSync(commandsDir) ? readdirSync(commandsDir).filter(f => f.endsWith(".md")) : [];
  console.log(`\nSensei plugin installed:`);
  console.log(`  Skills:   ${skills.length} (${skills.join(", ")})`);
  console.log(`  Commands: ${commands.length} (${commands.map(c => "/" + c.replace(".md", "")).join(", ")})`);
  console.log(`\nRestart Claude Code to activate.`);
}

install().catch((err) => {
  console.error("Install failed:", err.message);
  process.exit(1);
});
