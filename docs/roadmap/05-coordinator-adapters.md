# Coordinator Adapters — LLM and Agent Agnosticism

---

## Two Different Abstractions

Sensei works with two distinct external components that are often conflated:

| Term | What it is | Examples |
|---|---|---|
| **Model** | The LLM doing the reasoning | Claude (Anthropic), GPT-4o (OpenAI), Gemma (Google), Llama |
| **Coordinator** | The coding assistant tool the developer uses | Claude Code, opencode, GitHub Copilot, Kiro CLI, OpenAI Codex |

**Models** are already abstracted via `ModelBackend` in `packages/shared`.

**Coordinators** are currently hard-wired to Claude Code throughout the codebase
(config paths, hook scripts, MCP registration, skills location, session start mechanism).

This document defines the `CoordinatorAdapter` abstraction that replaces those
hard-wired assumptions and makes Claude Code the **default**, not the **only** option.

---

## What Is Coordinator-Specific

When sensei integrates with a coding assistant, five things vary by coordinator:

### 1. Context delivery — how does the agent get sensei's tools?

Most modern coordinators support MCP (Model Context Protocol). For those that do,
the MCP server (`packages/server`) is **unchanged** — only the registration location differs.

| Coordinator | Context delivery | MCP config location |
|---|---|---|
| Claude Code | MCP (stdio) | `~/.claude/mcp.json` or `<repo>/.mcp.json` |
| opencode | MCP (stdio) | `~/.opencode/mcp.json` |
| GitHub Copilot | MCP (VS Code ext API) | VS Code `settings.json` |
| Kiro CLI | MCP (likely) | TBD |
| OpenAI Codex | MCP or REST | TBD |

For coordinators without MCP, a fallback exists: **file-based context injection** —
sensei writes a `SENSEI.md` (or equivalent) containing a context snapshot that the
coordinator includes as a system prompt. Less dynamic, but universally compatible.

### 2. Session event capture — how does sensei observe the agent?

| Coordinator | Event mechanism | What sensei gets |
|---|---|---|
| Claude Code | Hooks (PreToolUse, PostToolUse) + OTLP | Tool events, token costs, turn boundaries |
| opencode | Hooks (different schema) | Tool events (format TBD) |
| GitHub Copilot | VS Code events (extension API) | Limited — no low-level tool hooks |
| Others | Diff-based polling fallback | What changed on disk (no session events) |

For coordinators with no hook support, sensei falls back to **git diff polling** —
less granular but still enables FTR scoring based on commit patterns.

### 3. Skill delivery — where are project skills installed?

| Coordinator | Skills location | Format |
|---|---|---|
| Claude Code | `~/.claude/skills/<repo>-*.md` | Markdown with YAML frontmatter |
| opencode | `~/.opencode/rules/<repo>-*.md` | Markdown (similar format) |
| GitHub Copilot | Injected via `.github/copilot-instructions.md` | Plain markdown |
| Generic fallback | `<repo>/.sensei/context.md` | Plain markdown summary |

### 4. Session start protocol — how is the session bootstrapped?

| Coordinator | Mechanism |
|---|---|
| Claude Code | `SessionStart` hook → injects `get_session_context` instruction as system-reminder |
| opencode | Equivalent hook (if supported) or AGENTS.md instruction |
| Others | File-based instruction in `CLAUDE.md` / `AGENTS.md` / `.cursorrules` equivalent |

### 5. Installation — how does sensei register itself?

`sensei init` needs to write the right config for whatever coordinator the developer uses.
Each adapter knows its own registration procedure.

---

## The CoordinatorAdapter Interface

The existing `AgentAdapter` (skills-only) is expanded to a full `CoordinatorAdapter`.
The existing interface is preserved as a subset.

```typescript
// packages/shared/src/coordinator.ts

export type EventCapture = 'hooks' | 'otlp' | 'polling' | 'extension-api';
export type ContextDelivery = 'mcp' | 'file-injection';

export interface CoordinatorAdapter {
  /** Unique identifier, used in config and logging */
  readonly name: string;

  /** Human-readable label */
  readonly displayName: string;

  /** How this coordinator receives context from sensei */
  readonly contextDelivery: ContextDelivery;

  /** How sensei captures session events from this coordinator */
  readonly eventCapture: EventCapture;

  /** Detect if this coordinator is installed on the current machine */
  isInstalled(): Promise<boolean>;

  /** Register the MCP server (or write context file) for this coordinator */
  installContextDelivery(serverPath: string, repoPaths: string[]): Promise<void>;

  /** Install event capture (hooks, OTLP config, extension registration) */
  installEventCapture(daemonUrl: string): Promise<void>;

  /** Write skill files in this coordinator's format and location */
  writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]>;

  /** List already-installed skill files for this repo */
  installedSkills(repoSlug: string): Promise<AgentSkillFile[]>;

  /** Configure the session start protocol */
  installSessionStart(instruction: string): Promise<void>;

  /** Write the coordinator-specific project context file (CLAUDE.md, AGENTS.md, etc.) */
  writeProjectContext(repoPath: string, context: ProjectContext): Promise<void>;

  /** Uninstall all sensei integrations for this coordinator */
  uninstall(repoSlug?: string): Promise<void>;
}
```

The `ClaudeAdapter` (currently in `packages/engine/src/agent/claude-adapter.ts`) becomes
the reference implementation of `CoordinatorAdapter`.

---

## Coordinator Registry

A registry maps coordinator name → adapter instance. `sensei init` uses it to:
1. Auto-detect which coordinators are installed
2. Let the developer choose which to configure (can configure multiple)
3. Run the full installation for the selected coordinators

```typescript
// packages/shared/src/coordinator-registry.ts

export class CoordinatorRegistry {
  private adapters: Map<string, CoordinatorAdapter>;

  register(adapter: CoordinatorAdapter): void;

  get(name: string): CoordinatorAdapter | undefined;

  /** Returns all adapters where isInstalled() === true */
  async detectInstalled(): Promise<CoordinatorAdapter[]>;

  /** The default adapter (Claude Code) */
  get default(): CoordinatorAdapter;
}
```

Adding a new coordinator adapter requires:
1. A new class implementing `CoordinatorAdapter`
2. One `registry.register(new TheirAdapter())` call
3. No changes anywhere else in the codebase

---

## MCP Server Is Coordinator-Agnostic

The MCP server (`packages/server`) does not change per coordinator.
It speaks the standard MCP protocol and the tools it exposes are useful to any
coordinator that supports MCP.

What changes: **where it is registered**. Each adapter writes the registration to the
right config file in the right format.

```typescript
// Claude Code: writes to ~/.claude/mcp.json
{
  "mcpServers": {
    "sensei": { "command": "bun", "args": ["..."] }
  }
}

// opencode: writes to ~/.opencode/mcp.json
{
  "mcpServers": {
    "sensei": { "command": "bun", "args": ["..."] }
  }
}

// File injection fallback (no MCP support): writes SENSEI.md to repo root
// Contains a snapshot of context, updated on each index run
```

---

## Session Tracking Without Hooks

For coordinators that do not support hooks (or where event capture is not yet
implemented), sensei falls back to **git-based session inference**:

- A session starts when the developer opens a project in sensei's workspace
- A session ends when they close it or switch projects
- FTR is inferred from commit patterns: commits that immediately follow a previous
  commit on the same work are counted as rework cycles
- Token costs are estimated from session duration and model (no OTLP data)

This is less precise than hook-based tracking but still useful.
The fallback is the baseline. Adapters can enhance it with coordinator-specific capture.

---

## Shipped Now vs. Later

### Ships with Phase 0 (foundation)

The `CoordinatorAdapter` interface and registry are defined. Claude Code adapter
is refactored to implement the full interface. The `AgentAdapter` in `packages/engine`
is replaced by `CoordinatorAdapter` in `packages/shared`.

This costs minimal extra work now and prevents the hard-wired Claude Code assumptions
from being baked deeper into the system.

### Ships when there is user demand

| Coordinator | Blocker before shipping |
|---|---|
| opencode | Verify hook schema and MCP config format |
| GitHub Copilot | MCP support via Copilot Extensions is in preview — wait for GA |
| Kiro CLI | AWS just launched — MCP format TBD |
| OpenAI Codex | MCP support announced — wait for stable API |
| Generic file injection | Already designed — trivial to ship as a fallback |

No adapter is shipped until it can be properly tested. A partial adapter
that fails silently is worse than no adapter.

### Selection in `sensei init`

```
? Which AI coding assistant do you use?
  ● Claude Code (default, fully supported)
  ○ opencode (beta)
  ○ GitHub Copilot (coming soon)
  ○ Other / None (file-based context only)
```

Selecting "coming soon" installs only the file-based fallback and notes that
full integration will be available in a future version.

---

## What This Means for the Desktop App

The Tauri desktop app shows coordinator status in the project settings view:

```
Coordinator integrations:
  ✓ Claude Code  — MCP registered, hooks active, OTLP receiving
  ○ opencode     — not detected
  ○ GitHub Copilot — coming soon
```

The developer can add, remove, or reconfigure any integration from the app.
This replaces the current `sensei init` / `sensei doctor` CLI flow for coordinator setup.

---

## Relationship to the Model Backend

`CoordinatorAdapter` and `ModelBackend` are independent abstractions:

```
Developer workflow
      │
      ▼
  Coordinator (Claude Code, opencode, Copilot...)
      │ uses MCP / extension API
      ▼
  Sensei (context delivery, session tracking)
      │ uses ModelBackend for inference tasks
      ▼
  Model (Claude, GPT-4o, Gemma, Llama...)
```

A developer can use Claude Code (coordinator) with GPT-4o (model) if their setup
supports it. Or opencode (coordinator) with Claude (model). Sensei does not care —
it works with whatever the developer has configured.
