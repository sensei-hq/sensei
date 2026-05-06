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
- `workaround` — temporary alternative when primary source unavailable. Marked for replacement — when the real source becomes available, discard the workaround.
- `feature_request` — link to upstream issue tracking the real solution

### 2. Configuration

```yaml
# .sensei/capabilities.yaml
capabilities:
  token_tracking:
    enabled: false
    source: session_hook          # the real solution: ACP provides token counts
    workaround:
      method: estimate_from_turns # avg tokens per turn × turn count
      discard_when: "anthropics/claude-code#11008 resolved"
    feature_request: "anthropics/claude-code#11008"

  tool_response_capture:
    enabled: true
    source: post_tool_hook        # the real solution: ACP includes response preview
    workaround:
      method: mcp_cache           # re-execute MCP calls via daemon, cache response
      limitation: "only works for MCP tools, not Bash/Read/Edit"
      discard_when: "anthropics/claude-code FR-2 resolved"
    feature_request: null         # not submitted — workaround sufficient for now

  quota_tracking:
    enabled: false
    source: cli_json              # the real solution: /cost --json or hook payload
    workaround:
      method: null                # no workaround currently
      discard_when: "anthropics/claude-code#50926 resolved"
    feature_request: "anthropics/claude-code#50926"

  task_lifecycle:
    enabled: true
    source: task_hooks            # the real solution: ACP PreTask/PostTask hooks
    workaround:
      method: session_start_closes_prev  # SessionStart finalizes previous session
      limitation: "session-level only, not per-task within a session"
      discard_when: "anthropics/claude-code#50931 resolved"
    feature_request: "anthropics/claude-code#50931"

  headless_benchmarks:
    enabled: false
    source: cli_headless          # the real solution: ACP headless mode
    workaround:
      method: manual_sessions     # user runs sessions manually, app tracks them
      limitation: "doesn't scale, introduces human variance"
      discard_when: "anthropics/claude-code#50927 resolved"
    feature_request: "anthropics/claude-code#50927"

  mindset_tracking:
    enabled: true
    source: command_events        # commands log mindset_applied via log_event()
    workaround: null              # this IS the real implementation, not a workaround

  persona_tracking:
    enabled: true
    source: command_events
    workaround: null

  rule_adherence:
    enabled: true
    source: command_events
    workaround: null
```

### 3. Registry in daemon

```rust
pub enum CapabilityStatus {
    Real,                    // primary source available — use it
    Workaround(String),      // temporary alternative — discard when real source lands
    Unavailable(String),     // not possible yet — show disabled state + tracking link
}

pub fn capability_status(cap: &str) -> CapabilityStatus {
    let c = CAPABILITIES.get(cap);
    match c {
        Some(c) if c.enabled && c.workaround.is_none() => CapabilityStatus::Real,
        Some(c) if c.enabled && c.workaround.is_some() => 
            CapabilityStatus::Workaround(c.workaround.limitation.clone()),
        Some(c) => CapabilityStatus::Unavailable(c.feature_request.clone()),
        None => CapabilityStatus::Unavailable("unknown".into()),
    }
}

// Data access adapts to status
pub fn get_token_count(session_id: &str) -> TokenResult {
    match capability_status("token_tracking") {
        Real => TokenResult::exact(store.get_session_tokens(session_id)),
        Workaround(_) => {
            let turns = store.count_session_turns(session_id);
            TokenResult::estimated(turns * AVG_TOKENS_PER_TURN)
        }
        Unavailable(fr) => TokenResult::unavailable(fr),
    }
}
```

### 4. Desktop adapts to capability status

```svelte
{#if tokenResult.kind === "exact"}
  <MetricCard label="Tokens" value={tokenResult.value} />
{:else if tokenResult.kind === "estimated"}
  <MetricCard label="Tokens" value={tokenResult.value} badge="est."
    hint="Estimated from turn count. Exact tracking pending: {tokenResult.trackingUrl}" />
{:else}
  <MetricCard label="Tokens" value="—" disabled
    hint="Pending ACP support"
    action={{ label: "Track issue", url: tokenResult.trackingUrl }} />
{/if}
```

Three visual states:
- **Real data** — clean display, no qualifier
- **Workaround data** — shown with "est." or "approx" badge + link to the issue that will replace it
- **Unavailable** — disabled with "Track issue" link so the user can watch/upvote

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
2. Remove the workaround config (`workaround: null`)
3. Run `sensei init` or `./scripts/install-plugin.sh` — picks up new profile
4. Capability switches from `Workaround` to `Real`
5. Dashboard shows real data, "est." badges disappear
6. Workaround code can be removed in a subsequent cleanup

**No application code changes needed** — the capability registry handles the switch. The workaround code stays inert (never called) until explicitly cleaned up.

### 7. Workaround lifecycle

```
Workaround created → used while FR is open → FR resolved → 
  ACP profile updated → capability switches to Real → 
  workaround code inert → cleanup PR removes workaround
```

Each workaround has a `discard_when` field that ties it to a specific upstream issue. When that issue closes, the workaround is stale and should be removed.

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
