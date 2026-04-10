import type { AgentSkillFile } from "@sensei/shared";

/**
 * AcpAdapter — integration interface for an Agent Coding Platform (ACP).
 *
 * An ACP is any AI coding tool that sensei integrates with:
 * Claude Code, Cursor, Windsurf, Kiro, Zed, etc.
 *
 * Each ACP has different locations and formats for skills, rules, MCP config,
 * hooks, plugins/commands, and settings injection.
 */
export interface AcpAdapter {
  /** Unique identifier matching the desktop setup wizard ACP list */
  readonly id: string;

  /** Human-readable name */
  readonly name: string;

  /** Absolute path to this ACP's skills/snippets directory */
  readonly skillsDir: string;

  /** Write skill markdown files for this ACP's format; returns written file list */
  writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]>;

  /** List skill files already installed for this repo slug */
  installedSkills(repoSlug: string): Promise<AgentSkillFile[]>;

  /** Register the senseid MCP server in this ACP's MCP config file */
  registerMcp?(senseiCmd: string): Promise<void>;

  /** Install hooks (pre/post tool use automation) — ACP-specific */
  installHooks?(): Promise<void>;

  /** Write global rules file (.cursorrules, .windsurfrules, etc.) */
  writeRules?(content: string, repoPath: string): Promise<string>;

  /** Install sensei plugin/extension if the ACP supports plugins */
  installPlugin?(): Promise<void>;

  /** Register sensei slash commands for this ACP */
  registerCommands?(commands: string[]): Promise<void>;

  /** Inject environment variables / endpoints into the ACP's settings */
  injectSettings?(opts: { otlpEndpoint?: string }): Promise<void>;

  /** Detect whether this ACP is installed on the current machine */
  detect(): Promise<boolean>;
}
