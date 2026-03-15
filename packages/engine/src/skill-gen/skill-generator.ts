import type { ModelBackend, ProjectProfile } from "@sensei/shared";
import type { SkillValidator } from "./skill-validator.js";

type SkillCategory = "orientation" | "workflow" | "context" | "patterns";

function buildPrompt(category: SkillCategory, profile: ProjectProfile): string {
  const slug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");

  if (category === "orientation") {
    return `Generate a skill file in SKILL.md format for the category "orientation".

Project: ${profile.repoName}
Language: ${profile.dominantLanguage}
Framework: ${profile.framework ?? "none"}
Packages: ${profile.packageNames.join(", ")}
Key exported symbols: ${profile.keySymbols.slice(0, 15).join(", ")}

The skill helps an AI agent orient themselves in this codebase. Include:
- What the project does
- Key packages and their responsibilities
- Most important symbols to know
- Where to start when beginning a new task

Output exactly this format and nothing else:

---
name: ${slug}-orientation
description: Use when starting work in the ${profile.repoName} repo to understand its structure and key components.
---

# ${profile.repoName} Orientation

[skill content here — 150-300 words, specific to this project]`;
  }

  if (category === "workflow") {
    const cmdList = Object.entries(profile.cliCommands)
      .slice(0, 10)
      .map(([k, v]) => `  ${k}: ${v}`)
      .join("\n");
    return `Generate a skill file in SKILL.md format for the category "workflow".

Project: ${profile.repoName}
Test pattern: ${profile.testPattern}
CLI commands:
${cmdList}
Sensei config:
${profile.senseiConfig.slice(0, 400)}

The skill teaches an AI agent the correct development workflow. Include:
- How to run tests (exact command)
- How to build/start the project
- Key CLI commands and when to use them
- TDD workflow for this specific project

Output exactly this format and nothing else:

---
name: ${slug}-workflow
description: Use when performing development tasks in ${profile.repoName} to follow the correct workflow.
---

# ${profile.repoName} Workflow

[skill content here — 150-300 words, specific to this project]`;
  }

  if (category === "context") {
    return `Generate a skill file in SKILL.md format for the category "context".

Project: ${profile.repoName}
Packages: ${profile.packageNames.join(", ")}
Key symbols: ${profile.keySymbols.join(", ")}
Sensei MCP tools available: get_session_context, search, load_context, context_pack

The skill teaches when and how to use sensei's context tools. Include:
- When to call get_session_context vs context_pack
- How to search for relevant code
- Token budget guidance
- Which key symbols to load for common task types in this project

Output exactly this format and nothing else:

---
name: ${slug}-context
description: Use when deciding how to load context for a task in ${profile.repoName}.
---

# ${profile.repoName} Context Loading

[skill content here — 150-300 words, specific to this project]`;
  }

  // patterns
  return `Generate a skill file in SKILL.md format for the category "patterns".

Project: ${profile.repoName}
Language: ${profile.dominantLanguage}
Framework: ${profile.framework ?? "none"}
Test pattern: ${profile.testPattern}
Available scripts: ${Object.keys(profile.cliCommands).join(", ")}

The skill documents coding conventions for this project. Include:
- File naming conventions
- Testing conventions (based on ${profile.testPattern})
- Error handling patterns
- Import/export patterns
- Framework-specific conventions for ${profile.framework ?? "this project"}

Output exactly this format and nothing else:

---
name: ${slug}-patterns
description: Use when writing code in ${profile.repoName} to follow established patterns and conventions.
---

# ${profile.repoName} Patterns

[skill content here — 150-300 words, specific to this project]`;
}

export class SkillGenerator {
  constructor(
    private readonly model: ModelBackend,
    private readonly profile: ProjectProfile,
    private readonly validator: SkillValidator,
  ) {}

  async generate(): Promise<Record<SkillCategory, string>> {
    const categories: SkillCategory[] = ["orientation", "workflow", "context", "patterns"];
    const results: Partial<Record<SkillCategory, string>> = {};

    for (const category of categories) {
      let skillMarkdown = await this.model.generate(buildPrompt(category, this.profile));

      for (let attempt = 0; attempt < 3; attempt++) {
        const { valid, issues } = await this.validator.validate(category, skillMarkdown);
        if (valid) break;

        if (attempt === 2) {
          throw new Error(
            `Failed to generate valid ${category} skill after 3 attempts. Issues: ${issues.join("; ")}`,
          );
        }

        // Retry with validator feedback
        const retryPrompt =
          buildPrompt(category, this.profile) +
          `\n\nPrevious attempt was rejected. Issues to fix:\n${issues.join("\n")}\nPlease address these issues in your response.`;
        skillMarkdown = await this.model.generate(retryPrompt);
      }

      results[category] = skillMarkdown;
    }

    return results as Record<SkillCategory, string>;
  }
}
