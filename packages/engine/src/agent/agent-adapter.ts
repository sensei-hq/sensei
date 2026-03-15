import type { AgentSkillFile } from "@sensei/shared";

export interface AgentAdapter {
  /** Absolute path to the agent's skills directory */
  readonly skillsDir: string;

  /** Write skill markdown files for each category; returns AgentSkillFile[] with paths + timestamps */
  writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]>;

  /** List skill files already written for this repo slug */
  installedSkills(repoSlug: string): Promise<AgentSkillFile[]>;
}
