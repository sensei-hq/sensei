---
name: Capability Registry
status: idea
origin: docs/ideas/24a-observatory-data-audit.md
date: 2026-04-19
description: Configurable capability system — enable/disable/configure features per ACP, gracefully degrade when source data unavailable
---

# Capability Registry

## Problem

Some observatory features depend on ACP capabilities that don't exist yet (token counts, task hooks, headless mode). We don't want to:
- Wait for ACPs to implement everything before building
- Hard-code availability assumptions that break across ACPs
- Ship features that silently fail when data is unavailable

## Solution

A **capability registry** — each feature declares what data it needs, the registry knows what's available, and the system gracefully enables/disables features based on what the current ACP provides.

## How it works

### 1. Capability definitions

Each capability has:
- `id` — unique key
- `description` — what this enables
- `enabled` — on/off
- `source` — where the data comes from (hook, api, cache, estimate)
- `fallback` — alternative when primary source unavailable
- `feature_request` — link to upstream issue tracking this

### 2. Configuration

```yaml
# .sensei/capabilities.yaml
capabilities:
  token_tracking:
    enabled: false
    source: session_hook
    fallback: estimate_from_turns
    feature_request: "anthropics/claude-code#11008"
    notes: "Enable when ACP exposes token counts in hooks"

  tool_response_capture:
    enabled: true
    source: mcp_cache
    fallback: null
    notes: "MCP tools re-executed via daemon cache. Non-MCP tools still opaque."

  quota_tracking:
    enabled: false
    source: cli_json
    fallback: null
    feature_request: "anthropics/claude-code#50926"
    notes: "Enable when /cost --json or equivalent available"

  task_lifecycle:
    enabled: true
    source: command_events
    fallback: session_start_closes_prev
    feature_request: "anthropics/claude-code#50931"
    notes: "Partial: commands log task events. Full PreTask/PostTask needs ACP support."

  headless_benchmarks:
    enabled: false
    source: cli_headless
    fallback: manual_sessions
    feature_request: "anthropics/claude-code#50927"
    notes: "Enable when ACP supports headless execution"

  mindset_tracking:
    enabled: true
    source: command_events
    fallback: null
    notes: "Commands log mindset_applied events via log_event() MCP tool"

  persona_tracking:
    enabled: true
    source: command_events
    fallback: null
    notes: "Commands log persona_applied events"

  rule_adherence:
    enabled: true
    source: command_events
    fallback: null
    notes: "Commands log rule_checked events"
```

### 3. Registry in daemon

```rust
// Capability check — used by dashboard, sessions, profiles pages
pub fn is_capable(cap: &str) -> bool {
    CAPABILITIES.get(cap).map(|c| c.enabled).unwrap_or(false)
}

// Graceful fallback
pub fn get_token_count(session_id: &str) -> Option<TokenData> {
    if is_capable("token_tracking") {
        // Read from session hook data
        store.get_session_tokens(session_id)
    } else if is_capable("token_tracking.fallback") {
        // Estimate from turn count
        let turns = store.count_session_turns(session_id);
        Some(TokenData::estimated(turns * AVG_TOKENS_PER_TURN))
    } else {
        None // UI shows "—" instead of a number
    }
}
```

### 4. Desktop adapts to capabilities

```svelte
{#if capabilities.token_tracking}
  <MetricCard label="Tokens" value={session.tokens_in} />
{:else if capabilities.token_tracking_fallback}
  <MetricCard label="Tokens (est.)" value={session.estimated_tokens} muted />
{:else}
  <MetricCard label="Tokens" value="—" disabled
    hint="Enable when ACP supports token tracking" />
{/if}
```

Disabled features show a clear explanation, not empty space. "Tokens: — (requires ACP support → [track issue](link))"

### 5. ACP profiles

Different ACPs have different capabilities. The registry loads an ACP profile at init:

```yaml
# marketplace/acp-profiles/claude-code.yaml
acp: claude-code
capabilities:
  session_start_hook: true
  pre_tool_hook: true
  post_tool_hook: true
  pre_compact_hook: true
  stop_hook: true
  user_prompt_hook: true
  token_counts: false          # FR-1
  tool_response_preview: false # FR-2
  quota_api: false             # FR-3
  headless_mode: false         # FR-4
  task_hooks: false            # FR-5
  mcp_support: true
  cost_command: true           # /cost exists but not machine-readable

# marketplace/acp-profiles/codex.yaml
acp: codex
capabilities:
  headless_mode: true          # codex exec exists
  token_counts: false          # FR pending
  mcp_support: false           # codex uses its own tool system
  structured_exec_output: false # FR-1 pending

# marketplace/acp-profiles/opencode.yaml
acp: opencode
capabilities:
  mcp_support: true
  session_start_hook: false    # TBD
  token_counts: false          # FR-1 pending
  headless_mode: false         # FR-2 pending
```

### 6. Upgrade path

When an ACP implements a requested feature:

1. Update the ACP profile (`claude-code.yaml`: `token_counts: true`)
2. Run `sensei init` or `./scripts/install-plugin.sh` — picks up new profile
3. Capability auto-enables
4. Dashboard starts showing real data where estimates were

No code changes needed — just profile config.

## What to build now

1. **Capability registry** in daemon — reads `.sensei/capabilities.yaml`, provides `is_capable()` API
2. **ACP profiles** in marketplace — one yaml per ACP with known capabilities
3. **`sensei init` enhancement** — detects ACP, copies appropriate profile
4. **Fallback implementations** — token estimation, session-start-closes-prev, MCP cache
5. **Desktop capability awareness** — UI adapts to what's available

## What this enables

- Build ALL observatory pages now, with graceful degradation
- Enable features instantly when ACPs ship updates
- Compare ACPs: "Claude Code has 7/10 capabilities, Codex has 4/10"
- Users see what's possible and what's pending — transparency, not mystery
