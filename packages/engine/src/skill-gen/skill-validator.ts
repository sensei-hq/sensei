import type { ModelBackend, ProjectProfile } from "@sensei/shared";

export class SkillValidator {
  constructor(
    private readonly model: ModelBackend,
    private readonly profile: ProjectProfile,
  ) {}

  async validate(
    category: string,
    skillMarkdown: string,
  ): Promise<{ valid: boolean; issues: string[] }> {
    const prompt = `You are reviewing a generated skill file for accuracy.

Project profile:
- Repo: ${this.profile.repoName}
- Packages: ${this.profile.packageNames.join(", ")}
- Key symbols: ${this.profile.keySymbols.slice(0, 10).join(", ")}
- CLI commands available: ${Object.keys(this.profile.cliCommands).join(", ")}
- Test pattern: ${this.profile.testPattern}

Skill category: ${category}

Generated skill:
---
${skillMarkdown}
---

Check this skill for accuracy. Verify:
1. Package names mentioned match the project profile
2. Commands referenced are real (exist in the profile's CLI commands)
3. No hallucinated file paths
4. YAML frontmatter is present with name and description fields

Reply with exactly one of:
- "VALID" if all checks pass
- A numbered list of issues (e.g. "1. Command 'yarn test' not found") if any checks fail`;

    const response = await this.model.generate(prompt);
    const trimmed = response.trim();

    if (trimmed === "VALID" || trimmed.startsWith("VALID")) {
      return { valid: true, issues: [] };
    }

    const issues = trimmed
      .split("\n")
      .filter(line => /^\d+\./.test(line.trim()))
      .map(line => line.trim());

    return {
      valid: false,
      issues: issues.length > 0 ? issues : [trimmed],
    };
  }
}
