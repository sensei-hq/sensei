// packages/cli/src/lib/skill-catalog.ts

export interface SkillCatalogEntry {
  name: string;
  description: string;
  recommended: boolean;
}

export const SKILL_CATALOG: SkillCatalogEntry[] = [
  {
    name: "zero-errors-policy",
    description: "Zero lint/test errors at all times — before and after every change",
    recommended: true,
  },
  {
    name: "managing-project-sessions",
    description: "Structured session protocol with context snapshots",
    recommended: true,
  },
  {
    name: "pattern-based-development",
    description: "Check PATTERNS.md before implementing — follow established recipes",
    recommended: true,
  },
  {
    name: "detecting-doc-drift",
    description: "Flag design docs that have drifted from the code",
    recommended: true,
  },
  {
    name: "identifying-patterns",
    description: "Discover and document recurring structural patterns",
    recommended: true,
  },
  {
    name: "decomposing-broad-tasks",
    description: "Break large requests into focused subtasks before starting",
    recommended: false,
  },
  {
    name: "managing-context",
    description: "Trim and refocus context when switching tasks",
    recommended: false,
  },
  {
    name: "running-agentic-sessions",
    description: "Protocols for running long autonomous sessions safely",
    recommended: false,
  },
  {
    name: "compressing-content",
    description: "Reduce token usage by compressing code representations",
    recommended: false,
  },
  {
    name: "indexing-codebase",
    description: "Index an unfamiliar codebase for efficient navigation",
    recommended: false,
  },
];
