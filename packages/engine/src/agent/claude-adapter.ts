import { homedir } from "os";
import { join } from "path";
import { mkdir, writeFile, readFile, readdir, stat, cp } from "fs/promises";
import { existsSync } from "fs";
import type { AgentSkillFile, LibSkillFile } from "@sensei/shared";
import type { AcpAdapter } from "./acp-adapter.js";

export class ClaudeAdapter implements AcpAdapter {
  readonly id = "claude-code";
  readonly name = "Claude Code";
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    // Use injected path for testing; default to ~/.claude/skills/
    // Note: os.homedir() is used — never expand '~' manually, fs functions don't handle it
    this.skillsDir = skillsDir ?? join(homedir(), ".claude", "skills");
  }

  async detect(): Promise<boolean> {
    return existsSync(join(homedir(), ".claude"));
  }

  async writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]> {
    await mkdir(this.skillsDir, { recursive: true });
    const result: AgentSkillFile[] = [];

    for (const [category, markdown] of Object.entries(skills)) {
      const fileName = `${repoSlug}-${category}.md`;
      const filePath = join(this.skillsDir, fileName);
      await writeFile(filePath, markdown, "utf-8");
      result.push({
        category: category as AgentSkillFile["category"],
        path: filePath,
        generatedAt: new Date().toISOString(),
      });
    }

    return result;
  }

  async writeLibSkill(
    libName: string,
    markdown: string,
    repoSlug: string,
  ): Promise<LibSkillFile> {
    await mkdir(this.skillsDir, { recursive: true });
    const fileName = `${repoSlug}-lib-${libName}.md`;
    const filePath = join(this.skillsDir, fileName);
    await writeFile(filePath, markdown, "utf-8");
    return { libName, path: filePath, generatedAt: new Date().toISOString() };
  }

  /**
   * Install the sensei plugin from pluginSrcDir into Claude Code's plugin directory.
   * Copies to ~/.claude/plugins/marketplaces/local/plugins/sensei/ and registers
   * in installed_plugins.json.
   */
  async installPlugin(pluginSrcDir: string): Promise<void> {
    const pluginJsonPath = join(pluginSrcDir, ".claude-plugin", "plugin.json");
    const pluginJson = JSON.parse(await readFile(pluginJsonPath, "utf-8"));
    const version: string = pluginJson.version ?? "1.0.0";

    const pluginsDir = join(homedir(), ".claude", "plugins");
    const installPath = join(pluginsDir, "marketplaces", "local", "plugins", "sensei");
    const installedPluginsPath = join(pluginsDir, "installed_plugins.json");

    await mkdir(installPath, { recursive: true });
    await cp(pluginSrcDir, installPath, { recursive: true, force: true });

    let registry: { version: number; plugins: Record<string, unknown[]> } = {
      version: 2,
      plugins: {},
    };
    if (existsSync(installedPluginsPath)) {
      try { registry = JSON.parse(await readFile(installedPluginsPath, "utf-8")); } catch {}
    }
    registry.plugins["sensei@local"] = [{
      scope: "user",
      installPath,
      version,
      installedAt: new Date().toISOString(),
      lastUpdated: new Date().toISOString(),
    }];

    await mkdir(pluginsDir, { recursive: true });
    await writeFile(installedPluginsPath, JSON.stringify(registry, null, 2));
  }

  async installedSkills(repoSlug: string): Promise<AgentSkillFile[]> {
    if (!existsSync(this.skillsDir)) return [];

    const entries = await readdir(this.skillsDir);
    const prefix = `${repoSlug}-`;
    const result: AgentSkillFile[] = [];

    for (const entry of entries) {
      if (!entry.startsWith(prefix) || !entry.endsWith(".md")) continue;
      const filePath = join(this.skillsDir, entry);
      const stats = await stat(filePath);
      const category = entry.slice(prefix.length, -".md".length) as AgentSkillFile["category"];
      result.push({ category, path: filePath, generatedAt: stats.mtime.toISOString() });
    }

    return result;
  }
}
