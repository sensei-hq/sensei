import { readdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { intro, outro, log, spinner } from "@clack/prompts";
import { ClaudeAdapter } from "@sensei/engine";

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

  const adapter = new ClaudeAdapter();
  if (!await adapter.detect()) {
    log.warn("Claude Code not detected (~/.claude not found). Plugin not installed.");
    process.exit(1);
  }

  const s = spinner();
  s.start(`Installing plugin into ${adapter.name}`);
  await adapter.installPlugin(pluginSrc);
  s.stop("Plugin files installed");

  log.success(`Registered sensei@local in installed_plugins.json`);

  // Summary from plugin source
  const skillsDir = join(pluginSrc, "skills");
  const commandsDir = join(pluginSrc, "commands");
  const skills = existsSync(skillsDir)
    ? (await readdir(skillsDir, { withFileTypes: true })).filter(e => e.isDirectory()).map(e => e.name)
    : [];
  const commands = existsSync(commandsDir)
    ? (await readdir(commandsDir)).filter(f => f.endsWith(".md")).map(f => "/" + f.replace(".md", ""))
    : [];

  log.info(`Skills (${skills.length}): ${skills.join(", ")}`);
  log.info(`Commands (${commands.length}): ${commands.join(", ")}`);

  outro("Done. Restart Claude Code to activate the sensei plugin.");
}
