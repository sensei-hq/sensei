/**
 * BaseAcpAdapter — shared implementation for file-based ACP adapters.
 *
 * Provides writeSkills, writeLibSkill, and installedSkills with a
 * configurable file extension (.md for most, .mdc for Cursor).
 * Concrete adapters only need to set id, name, skillsDir, skillExt,
 * and implement detect() plus any ACP-specific methods.
 */
import { join } from "path";
import { existsSync } from "fs";
import { mkdir, writeFile, readdir, stat } from "fs/promises";
import type { AgentSkillFile, LibSkillFile } from "@sensei/shared";
import type { AcpAdapter } from "./acp-adapter.js";

export abstract class BaseAcpAdapter implements AcpAdapter {
  abstract readonly id: string;
  abstract readonly name: string;
  abstract readonly skillsDir: string;

  /** File extension for skill files: ".md" (default) or ".mdc" (Cursor) */
  protected readonly skillExt: string = ".md";

  abstract detect(): Promise<boolean>;

  async writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]> {
    await mkdir(this.skillsDir, { recursive: true });
    const result: AgentSkillFile[] = [];
    for (const [category, markdown] of Object.entries(skills)) {
      const fileName = `${repoSlug}-${category}${this.skillExt}`;
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

  async writeLibSkill(libName: string, markdown: string, repoSlug: string): Promise<LibSkillFile> {
    await mkdir(this.skillsDir, { recursive: true });
    const fileName = `${repoSlug}-lib-${libName}${this.skillExt}`;
    const filePath = join(this.skillsDir, fileName);
    await writeFile(filePath, markdown, "utf-8");
    return { libName, path: filePath, generatedAt: new Date().toISOString() };
  }

  async installedSkills(repoSlug: string): Promise<AgentSkillFile[]> {
    if (!existsSync(this.skillsDir)) return [];
    const entries = await readdir(this.skillsDir);
    const prefix = `${repoSlug}-`;
    const result: AgentSkillFile[] = [];
    for (const entry of entries) {
      if (!entry.startsWith(prefix) || !entry.endsWith(this.skillExt)) continue;
      const filePath = join(this.skillsDir, entry);
      const stats = await stat(filePath);
      const category = entry.slice(prefix.length, -this.skillExt.length) as AgentSkillFile["category"];
      result.push({ category, path: filePath, generatedAt: stats.mtime.toISOString() });
    }
    return result;
  }
}
