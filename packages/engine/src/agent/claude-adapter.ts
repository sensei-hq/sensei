import { homedir } from "os";
import { join } from "path";
import { mkdir, writeFile, readdir, stat } from "fs/promises";
import { existsSync } from "fs";
import type { AgentSkillFile } from "@sensei/shared";
import type { AgentAdapter } from "./agent-adapter.js";

export class ClaudeAdapter implements AgentAdapter {
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    // Use injected path for testing; default to ~/.claude/skills/
    // Note: os.homedir() is used — never expand '~' manually, fs functions don't handle it
    this.skillsDir = skillsDir ?? join(homedir(), ".claude", "skills");
  }

  async writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]> {
    await mkdir(this.skillsDir, { recursive: true });
    const result: AgentSkillFile[] = [];

    for (const [category, markdown] of Object.entries(skills)) {
      const fileName = `sensei-${repoSlug}-${category}.md`;
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

  async installedSkills(repoSlug: string): Promise<AgentSkillFile[]> {
    if (!existsSync(this.skillsDir)) return [];

    const entries = await readdir(this.skillsDir);
    const prefix = `sensei-${repoSlug}-`;
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
