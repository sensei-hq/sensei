# Gaps and Open Decisions

> Identified by reading the actual codebase against the roadmap. These are real blockers
> and decisions that must be resolved before or during implementation — not hypothetical ones.

---

## Gap 1 — Skills have two separate concepts that are conflated

**What exists in code:**

Two completely different things are both called "skills" and both land in `~/.claude/skills/`:

| Type | Origin | Example | Location |
|---|---|---|---|
| **Practice skills** | Bundled in `packages/cli/skills/` — sensei's own best practices | `zero-errors-policy.md`, `managing-project-sessions.md` | Copied to `~/.claude/skills/` |
| **Generated skills** | LLM-generated per-repo by `SkillGenerator` — describes the specific project | `myrepo-context.md`, `myrepo-patterns.md` | Written to `~/.claude/skills/` by `ClaudeAdapter` |

These need different delivery strategies per coordinator:

- Practice skills: may be injected as a system prompt block, or written to a coordinator-specific rules file (`~/.opencode/rules/`, `.github/copilot-instructions.md`)
- Generated skills: per-repo context that the coordinator needs to load when in that project

**Decision needed:** Should the `CoordinatorAdapter` handle both types, or should practice skill delivery be separate from generated skill delivery? The content is generic; only the delivery mechanism is coordinator-specific.

---

## Gap 2 — Hook installation is entirely Claude Code-specific and not abstracted

**What exists in code (`packages/collector/src/install.ts`):**

- Writes Bun scripts (`sensei-pre-tool-use.ts`, `sensei-post-tool-use.ts`) to `~/.claude/hooks/`
- Modifies `~/.claude/settings.json` to register `PreToolUse` and `PostToolUse` hooks
- Writes a launchd plist for the collector daemon (macOS only)
- Hardcodes `bun` as the runtime in hook scripts

**What this means for coordinator adapters:**

- `opencode`: different hook schema, different settings file, `bun` may not be available
- `GitHub Copilot`: no hook system — event capture must use VS Code extension API or polling
- `Kiro / Codex`: hook support unknown
- **Windows**: launchd is macOS-only; no systemd or Task Scheduler equivalent exists

The collector daemon itself (port 51789, event ingestion) is generic. It is the **hook scripts that deliver to it** that are coordinator-specific. That separation is not reflected in the code — `installHooks()` in the collector does both.

**Decision needed:**
1. Split the collector daemon lifecycle management (launchd/systemd/Tauri) from hook script installation (coordinator-specific).
2. Hook script installation becomes part of `CoordinatorAdapter.installEventCapture()`.
3. What is the correct fallback for coordinators with no hook support? Git-diff polling (documented) or something else?

---

## Gap 3 — MCP registration is hardcoded to `~/.claude/mcp.json`

**What exists in code (`packages/cli/src/commands/setup.ts`):**

```typescript
const MCP_CONFIG = join(homedir(), ".claude", "mcp.json");
// ...
mcpConfig.mcpServers["sensei"] = {
  command: "bun",
  args: [entryPath],
  env: { SENSEI_REPO_PATH: repoPath },
};
await writeFile(MCP_CONFIG, JSON.stringify(mcpConfig, null, 2), "utf-8");
```

Known MCP config paths per coordinator:

| Coordinator | MCP config location | Format |
|---|---|---|
| Claude Code | `~/.claude/mcp.json` | `{ mcpServers: { name: { command, args, env } } }` |
| opencode | `~/.config/opencode/config.json` | Likely same MCP structure |
| Codex | `~/.codex/config.toml` | TOML, different structure |
| GitHub Copilot | VS Code `settings.json` → `mcp.servers` | JSON, VS Code-specific |
| Kiro | Unknown | TBD |

`setupMcp()` also hardwrites `OTEL_EXPORTER_OTLP_ENDPOINT` into `~/.claude/settings.json`.
That env var is Claude Code-specific. Other coordinators don't use OTLP.

**Decision needed:** `CoordinatorAdapter.installContextDelivery()` must handle both MCP config format differences and the coordinator-specific env/settings injection. Where does the OTLP setup live when not Claude Code?

---

## Gap 4 — `setupAgent` hard-rejects non-Claude agents

**What exists in code:**

```typescript
export async function setupAgent(repoPath: string, agent: string): Promise<void> {
  if (agent !== "claude") {
    console.error(`Agent '${agent}' is not yet supported. Supported: claude`);
    process.exit(1);
  }
```

This is the entry point for coordinator setup. It is a hard block. Before any other coordinator
work can be done, this function must be refactored to use the `CoordinatorRegistry`.

This is a Phase 0 task, not Phase 1. If it is left, it will be the source of confusion
for anyone who reads the new roadmap docs and tries to add a new coordinator.

---

## Gap 5 — Project context file (CLAUDE.md) is Claude-specific in content, not just name

**What exists in code (`packages/cli/src/templates/claude-md.ts`):**

The `CLAUDE.md` template hardcodes Claude Code MCP tool call syntax:
```
get_session_context(task_description="...")
context_pack(query)
checkpoint(outcome="...")
```

These are fine for Claude Code. For opencode or Codex the tool call syntax is different.
For GitHub Copilot the mechanism is different entirely.

**Three separate questions:**
1. **File name**: `CLAUDE.md` (Claude Code), `AGENTS.md` (opencode/Codex), `.rules` (Cursor), `KIRO.md` (Kiro) — each coordinator reads a different file
2. **Content**: The session protocol instructions reference coordinator-specific tool names
3. **Multiple coordinators**: If a developer uses both Claude Code and opencode on the same repo, do both files coexist? (Yes — they should, and sensei should manage both)

`AGENTS.md` is already written by `init.ts` alongside `CLAUDE.md`, but its content is generic
and doesn't include the session protocol. This is inconsistent — opencode users won't get
the session protocol they need.

**Decision needed:** Each `CoordinatorAdapter` should own its project context template. `writeProjectContext()` generates the right file with the right content. `init.ts` calls this for each installed coordinator, not a hardcoded list.

---

## Gap 6 — SQLite schema is not designed

**What the roadmap says:** Replace Supabase with SQLite.

**What exists:** Supabase migrations in `supabase/migrations/`. No SQLite schema.

The SQLite design from the roadmap (`02-local-architecture.md`) specifies two databases:
- `~/.sensei/sensei.db` — global (projects, ideas, cards, sessions, FTR, libraries, settings)
- `<repo>/.sensei/index.db` — per-repo (symbols, call_edges, imports, embeddings)

But the actual tables, columns, indexes, and constraints are not designed anywhere.
This is the foundation for everything in Phase 0. Without a schema:
- The engine rewrite (Supabase → SQLite) cannot start
- The collector rewrite cannot start
- The card system (Phase 3) has no target to build toward

**What needs to happen:** A `docs/roadmap/08-sqlite-schema.md` that defines both databases completely — tables, columns, types, indexes, foreign keys. This should be written before Phase 0 implementation starts.

---

## Gap 7 — `AgentAdapter` vs `CoordinatorAdapter` — two interfaces, one class, wrong package

**What exists:**

```
packages/engine/src/agent/agent-adapter.ts   — interface AgentAdapter (skills only)
packages/engine/src/agent/claude-adapter.ts  — class ClaudeAdapter implements AgentAdapter
```

**What the roadmap defines:**

```
packages/shared/src/coordinator.ts           — interface CoordinatorAdapter (full contract)
packages/shared/src/coordinator-registry.ts  — class CoordinatorRegistry
```

These don't exist in code yet. And the existing `AgentAdapter` + `ClaudeAdapter` are in
`packages/engine`, but the roadmap puts `CoordinatorAdapter` in `packages/shared`.

**Why this matters:** `packages/collector` currently installs hooks independently of `packages/engine`'s `AgentAdapter`. They are two separate subsystems that both need to be coordinator-aware, but neither knows about the other's coordinator assumption. Unifying under `CoordinatorAdapter` in `packages/shared` makes both subsystems use the same adapter.

**Decision needed:**
1. Move `AgentAdapter` + `ClaudeAdapter` → `packages/shared` as `CoordinatorAdapter` + `ClaudeCoordinator`
2. `packages/collector`'s `installHooks()` becomes `ClaudeCoordinator.installEventCapture()`
3. `packages/cli`'s `install-skills.ts` becomes `coordinator.writeSkills()`
4. `packages/cli`'s `setup.ts` (setupMcp, setupAgent) becomes `coordinator.installContextDelivery()`
5. The `CLAUDE.md` / `AGENTS.md` templates become `coordinator.writeProjectContext()`

This is a significant refactor but it must happen in Phase 0, not Phase 2 — or the Phase 1 and 2
work will bake in more Claude-specific assumptions.

---

## Gap 8 — Daemon lifecycle: launchd vs Tauri vs CLI conflict

**What exists:** `installHooks()` writes a launchd plist for macOS. No equivalent for Linux or Windows.

**What the roadmap says:** The Tauri app manages the daemon lifecycle.

**The conflict:** Once Tauri ships (Phase 1), there will be two competing daemon lifecycle managers: launchd (installed by `sensei init`) and Tauri (manages it as a child process). These will fight.

**Unresolved questions:**
1. Pre-Tauri (Phases 0): launchd remains. Fine.
2. Post-Tauri (Phase 1+): Does Tauri replace launchd, or coexist?
3. What about headless use (CI, remote dev, no Tauri app running)? The CLI must still be able to start the daemon without Tauri.
4. Linux and Windows: launchd is macOS-only. What is the pre-Tauri daemon autostart for those platforms?

**Proposed resolution:** Three daemon start modes, all valid:
- **Tauri-managed**: Tauri starts/stops the daemon as a child process (primary for desktop use)
- **CLI-managed**: `sensei serve` starts it in foreground (CI and headless)
- **System-managed**: Optional launchd/systemd registration via `sensei serve --install` (advanced users only)

The current `installHooks()` installs launchd unconditionally. It should be opt-in.

---

## Gap 9 — OTLP telemetry is Claude Code-specific with no designed fallback

**What exists:** The OTLP endpoint (`localhost:51789`) is written into `~/.claude/settings.json`.
Claude Code sends `claude_code.api_request` events which carry token counts and costs.

**For other coordinators:** No OTLP. The collector daemon currently has no fallback for
token cost tracking when OTLP events don't arrive.

The roadmap mentions "git-based session inference" as a fallback but it is not designed:
- How is a session boundary detected without hooks?
- How is FTR scored without per-turn tool events?
- How is token cost estimated without OTLP data?

**Decision needed:** Define the minimum viable session tracking for non-OTLP coordinators.
Proposed: session = time between first and last file modification in a work period. FTR =
commit pattern (immediate revert or fixup commit = rework). Token cost = not tracked (shown as "—").

---

## Gap 10 — Skill content format compatibility across coordinators

**What exists:** Skills are markdown files with YAML frontmatter:
```markdown
---
name: zero-errors-policy
description: ...
triggers: [...]
---
# Zero Errors Policy
...
```

**The problem:** Different coordinators read skills differently:
- Claude Code: reads `~/.claude/skills/*.md`, uses frontmatter `name` and `triggers`
- opencode: reads `~/.opencode/rules/*.md` — frontmatter schema may differ
- GitHub Copilot: reads `.github/copilot-instructions.md` — single file, no frontmatter
- Kiro: reads `.kiro/steering/*.md` — different frontmatter

A skill written for Claude Code may not be parseable by opencode without modification.

**Decision needed:** Should sensei maintain coordinator-specific copies of skill content, or transform a single canonical format at installation time? Transformation at install time is simpler and avoids content duplication. The `CoordinatorAdapter.writeSkills()` method can apply the transformation.

---

## Summary: What Must Be Resolved Before Phase 0 Starts

| # | Gap | Blocking what | Proposed resolution |
|---|---|---|---|
| 1 | Two types of "skills" conflated | CoordinatorAdapter design | Separate `PracticeSkill` from `GeneratedSkill` in the adapter interface |
| 2 | Hooks not abstracted | Phase 0 refactor | Split daemon lifecycle from hook installation; hooks become `installEventCapture()` |
| 3 | MCP config hardcoded | Phase 0 refactor | Move to `CoordinatorAdapter.installContextDelivery()` |
| 4 | setupAgent hard-rejects non-Claude | Phase 0 refactor | Replace with registry lookup — 3 lines of change |
| 5 | CLAUDE.md is content-specific | Phase 0 + coordinator design | Templates become `CoordinatorAdapter.writeProjectContext()` |
| **6** | **SQLite schema not designed** | **Phase 0 cannot start** | **Write `08-sqlite-schema.md` first** |
| **7** | **AgentAdapter vs CoordinatorAdapter** | **Phase 0 cannot start** | **Define interface in shared, migrate ClaudeAdapter** |
| 8 | Daemon lifecycle conflict (launchd vs Tauri) | Phase 1 | Three modes; launchd becomes opt-in |
| 9 | OTLP fallback not designed | Non-Claude coordinator tracking | Define minimum session tracking for no-OTLP cases |
| 10 | Skill format compatibility | Multi-coordinator skill delivery | Transform at install time via adapter |

**Hard blockers (must resolve before writing any code):** Gaps 6 and 7.
**Should resolve in Phase 0 implementation:** Gaps 1, 2, 3, 4, 5, 10.
**Can resolve in Phase 1:** Gaps 8 and 9.
