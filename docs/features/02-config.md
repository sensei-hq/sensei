---
name: Configuration
type: feature
kind: functional
---

# Configuration

Configuration is everything sensei can be told, reachable anytime after [Setup](01-setup.md) — it is not part of the entry gate. These a free-navigation Settings surface: search or browse, change what you want, and it applies live.

The surface is grouped the way the settings rail is:

- **You** — your profile and the assistants sensei talks to.
- **Sources** — where code and docs come from (roots · projects · libraries).
- **Reasoning** — models, providers, and which model handles which role.
- **Extensions** — the plugins, skills, commands, and agents sensei packages.
- **Dōjō** — connecting to a shared team/org plane and sharing into it.

By default everything stays local. Nothing is shared until the user opts in.

## Flows

1. **Edit anytime.** Open Settings from the app, search or browse, change a
   setting — it takes effect without a restart.
2. **Re-scan.** Roots can be re-scanned to pick up new, moved, or renamed repos.
3. **Join a dōjō.** Connect by invitation/dōjō URL, or accept a dōjō surfaced by
   the org auto-discovery in Setup; membership is validated by the GitHub org or
   admin approval.

## Mockups

- [Setup wizard — the full stage set (assistants · profile · projects · libraries · instruments · routers · assignments)](../mockups/Sensei/lib/setup/setup-wizard.jsx)
- [Routers — local + cloud model inference](../mockups/Sensei/lib/setup/wiz-inference.jsx) · [inference settings](../mockups/Sensei/lib/setup/inference-settings.jsx)
- [Assignments — which model handles which role](../mockups/Sensei/lib/setup/wiz-assignments.jsx)
- [Collective settings — sharing into a dōjō](../mockups/Sensei/lib/observatory/collective-settings.jsx) · [share hub](../mockups/Sensei/lib/observatory/share-hub.jsx)
- [Dōjō in-app](../mockups/Sensei/lib/dojo/dojo-inapp.jsx)

## What's involved

> A breakdown of each area. `- [x]` done · `- [~]` partial · `- [ ]` not started.

### You

**General — profile & preferences**

- [x] Derive a display name from the home directory (`/Users/keiko` → `Keiko`)
- [x] Choose how forward sensei is — correction aggressiveness, digest cadence, regression nudges
- [x] Telemetry off by default
- [x] Local-first by default — nothing shared unless opted in

**Assistants**

- [x] Detect the AI tools already installed (Claude Code, Claude Desktop, …)
- [x] One switch per assistant — all-or-nothing register
- [x] Registers every part the assistant supports: plugins, skills, commands, agents, logging, metrics, and the MCP server
- [x] Per-part progress with retry on failure (e.g. a permission-denied path)
- [~] Fine-tune individual parts

### Sources

**Roots**

- [x] Add / remove the top-level folders the code lives in — recursive
- [x] Exclusions per root
- [x] Re-scan to pick up new, moved, or renamed repos

**Projects**

- [x] Group repos into projects/solutions; split or merge
- [x] Assign a role per repo
- [~] Project metadata — status, client, goal

**Libraries**

- [x] Index docs & code for libraries without their own MCP — sensei wraps them with its own tools
- [~] Add extra libraries by name / URL / language

### Reasoning

**Instruments (MCP registry)**

- [x] MCP tools recommended for the stack; install / enable
- [~] Playground — try the tools the way the LLM does, to see what works

**Providers**

- [x] Cloud provider API keys (Keychain-backed)

**Inference (routers + assignments)**

- [x] Local routers — Ollama and embedded models
- [x] Cloud routers via the configured providers
- [x] Which model handles which role (assignments are a live editor here, not a separate screen)
- [~] Routing strategy — MoE / deliberation

### Extensions

- [x] Package and manage plugins, skills, commands, and agents (marketplace)

### Dōjō (shared plane)

- [~] Connect to a dōjō by invitation URL or dōjō URL
- [~] Accept a dōjō surfaced by the org auto-discovery in [Setup](01-setup.md)
- [ ] Validate membership via the GitHub org or admin approval
- [~] Sharing — contribute learnings, review before sharing, on a schedule; download the collective
- [~] Pull shared standards / guidelines / principles and apply them to shared projects

## Status

| Feature | Status | Notes |
|---|---|---|
| General (profile · preferences · telemetry) | Done | settings/general + preferences-form |
| Assistants | Done | registers plugins · skills · commands · agents · logging · metrics · MCP |
| Roots | Done | add/remove, recursive, exclusions, re-scan |
| Projects | Done | group/split/merge, roles; metadata partial |
| Libraries | Done | index docs & code for MCP-less libs; add-extras partial |
| Instruments | Partial | MCP registry shipped; playground partial |
| Providers | Done | cloud API keys, Keychain-backed |
| Inference (routers + assignments) | Done | local + cloud, role→model live editor; routing strategy partial |
| Extensions | Done | plugins/skills/commands/agents packaging |
| Dōjō connect / join | Partial | connect + connections screens shipped; membership validation not built |
| Dōjō sharing (collective) | Partial | contribute/review/schedule/download; pull-and-apply standards partial |
</content>
