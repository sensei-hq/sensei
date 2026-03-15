// packages/engine/src/lib/lib-skill-generator.ts
import type { ModelBackend, ProjectProfile, LibEntry, DocPage } from "@sensei/shared";
import type { SkillValidator } from "../skill-gen/skill-validator.js";

function buildPrompt(entry: LibEntry, pages: DocPage[], profile: ProjectProfile): string {
  const slug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const topSections = pages.slice(0, 20).map(p => `- ${p.title}: ${p.description}`).join("\n");
  const relevantSymbols = profile.keySymbols
    .filter(s => s.toLowerCase().includes(entry.name.toLowerCase()))
    .join(", ");

  return `Generate a skill file in SKILL.md format for the library "${entry.name}".

Project: ${profile.repoName}
Library description: ${entry.description ?? entry.name}
Relevant project symbols: ${relevantSymbols || "none detected"}
Project config: ${profile.senseiConfig.slice(0, 300)}

Top documentation sections:
${topSections || "(no sections available)"}

Teach an AI agent how to use "${entry.name}" in the context of the ${profile.repoName} project.
Include: what the library does, key APIs and components, usage patterns for this project.

Output exactly this format:

---
name: ${slug}-lib-${entry.name}
description: Use when working with ${entry.name} in the ${profile.repoName} project.
---

# ${entry.name} Library Guide

[skill content here — 150-300 words]`;
}

export class LibSkillGenerator {
  constructor(
    private readonly model: ModelBackend,
    private readonly profile: ProjectProfile,
    private readonly validator: SkillValidator,
  ) {}

  async generate(entry: LibEntry, pages: DocPage[]): Promise<string> {
    const category = `lib-${entry.name}`;
    let skillMarkdown = await this.model.generate(buildPrompt(entry, pages, this.profile));

    for (let attempt = 0; attempt < 3; attempt++) {
      const { valid, issues } = await this.validator.validate(category, skillMarkdown);
      if (valid) return skillMarkdown;

      if (attempt === 2) {
        throw new Error(
          `Failed to generate valid ${entry.name} skill after 3 attempts. Issues: ${issues.join("; ")}`,
        );
      }

      const retryPrompt =
        buildPrompt(entry, pages, this.profile) +
        `\n\nPrevious attempt was rejected. Fix these issues:\n${issues.join("\n")}`;
      skillMarkdown = await this.model.generate(retryPrompt);
    }

    return skillMarkdown;
  }
}
