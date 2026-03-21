import { rm, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const PLUGIN_NAME = "sensei";
const PLUGIN_KEY = `${PLUGIN_NAME}@local`;

const claudeDir = join(homedir(), ".claude");
const pluginsDir = join(claudeDir, "plugins");
const installPath = join(pluginsDir, "cache", PLUGIN_NAME, "local");
const installedPluginsPath = join(pluginsDir, "installed_plugins.json");

async function uninstall() {
  // Remove installed files
  if (existsSync(installPath)) {
    await rm(installPath, { recursive: true, force: true });
    console.log(`✓ Removed ${installPath}`);
  } else {
    console.log(`Nothing to remove at ${installPath}`);
  }

  // Remove from registry
  if (existsSync(installedPluginsPath)) {
    const raw = await readFile(installedPluginsPath, "utf-8");
    const registry = JSON.parse(raw);
    if (registry.plugins[PLUGIN_KEY]) {
      delete registry.plugins[PLUGIN_KEY];
      await writeFile(installedPluginsPath, JSON.stringify(registry, null, 2));
      console.log(`✓ Deregistered ${PLUGIN_KEY}`);
    }
  }

  console.log(`\nSensei plugin uninstalled. Restart Claude Code to deactivate.`);
}

uninstall().catch((err) => {
  console.error("Uninstall failed:", err.message);
  process.exit(1);
});
