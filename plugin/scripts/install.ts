import { cp, mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync, readdirSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const PLUGIN_NAME = "sensei";
const PLUGIN_VERSION = "1.0.0";
const MARKETPLACE = "local";
const PLUGIN_KEY = `${PLUGIN_NAME}@${MARKETPLACE}`;

const __dirname = dirname(fileURLToPath(import.meta.url));
const pluginSrc = resolve(__dirname, "..");
const claudeDir = join(homedir(), ".claude");
const pluginsDir = join(claudeDir, "plugins");
const marketplacePath = join(pluginsDir, "marketplaces", MARKETPLACE);
const marketplacePluginPath = join(marketplacePath, "plugins", PLUGIN_NAME);
const marketplaceManifest = join(marketplacePath, ".claude-plugin", "marketplace.json");

async function install() {
  // 1. Create/update the local marketplace structure
  await mkdir(join(marketplacePath, ".claude-plugin"), { recursive: true });
  await mkdir(marketplacePluginPath, { recursive: true });

  // Write marketplace.json if it doesn't exist
  if (!existsSync(marketplaceManifest)) {
    await writeFile(marketplaceManifest, JSON.stringify({
      "$schema": "https://anthropic.com/claude-code/marketplace.schema.json",
      "name": MARKETPLACE,
      "description": "Locally installed plugins",
      "owner": { "name": "local" },
      "plugins": [],
    }, null, 2));
  }

  // Upsert sensei entry in marketplace.json
  const raw = await readFile(marketplaceManifest, "utf-8");
  const manifest = JSON.parse(raw);
  const pluginEntry = {
    name: PLUGIN_NAME,
    description: "Dev workflow skills and commands — cross-project guardrails plus project-specific opt-ins",
    version: PLUGIN_VERSION,
    author: { name: "Jerry" },
    source: `./plugins/${PLUGIN_NAME}`,
    category: "development",
  };
  const idx = manifest.plugins.findIndex((p: { name: string }) => p.name === PLUGIN_NAME);
  if (idx >= 0) manifest.plugins[idx] = pluginEntry;
  else manifest.plugins.push(pluginEntry);
  await writeFile(marketplaceManifest, JSON.stringify(manifest, null, 2));

  // Copy plugin files to marketplace entry
  await cp(pluginSrc, marketplacePluginPath, { recursive: true, force: true });
  console.log(`✓ Updated marketplace entry at ${marketplacePluginPath}`);

  // 2. Register marketplace (idempotent — ignore error if already registered)
  try {
    execFileSync("claude", ["plugin", "marketplace", "add", marketplacePath, "--scope", "user"], { stdio: "pipe" });
  } catch {
    // Already registered — fine
  }

  // 3. Install plugin via CLI
  execFileSync("claude", ["plugin", "install", PLUGIN_KEY, "--scope", "user"], { stdio: "pipe" });
  console.log(`✓ Installed ${PLUGIN_KEY} via claude CLI`);

  // 4. Summary
  const skillsDir = join(marketplacePluginPath, "skills");
  const commandsDir = join(marketplacePluginPath, "commands");
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
