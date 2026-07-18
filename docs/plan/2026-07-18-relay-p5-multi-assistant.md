---
name: Relay P5 — multi-assistant adapters
description: Phased plan for relay-engine P5 — a run-driver adapter layer so the daemon supervises assistants beyond Claude (ACP + fallback), normalized to the Execution→Segment→Item relay view. Open decisions flagged for Jerry.
date: 2026-07-18
status: planning (awaiting Jerry's plan/spec decisions before build)
---

# Relay P5 — multi-assistant adapters

**Goal:** the relay engine drives + supervises a run on **any** backend — Claude
Code (built), ACP-speaking assistants (Zed + adopters), and coarse fallbacks
(aider / Codex / plain CLIs) — through **one internal run-driver contract**, all
normalized to §6's `Execution → Segment → Item` view and the one control surface
(start · pause · gate · nudge · observe). It **never builds an agent runtime** (D7)
— it supervises + normalizes existing assistants.

Design source: [`relay-engine.md`](relay-engine.md) §7 (the adapter layer + the
Hooks/ACP/Fallback capability ladder) + §5 (control channel). Sequencing was
deferred **post-stable** on purpose (2026-07-16): P0–P4 shipped riding Claude
Code directly; P5 pays the multi-assistant abstraction cost only now that the
single-assistant product is proven + released (v0.4.0).

## Where we start (surveyed 2026-07-18)

- **The existing `crates/senseid/src/assistants/` trait is the WRONG seam for P5.**
  `assistants/trait_def.rs::Assistant` (+ `mod.rs`, `claude_code.rs`, `mcp_file.rs`,
  `watchdog.rs`, `health.rs`) is the **config/capture** layer — detect an assistant,
  wire sensei's MCP into its config, upgrade, capture-health. It has NO notion of
  *driving a run* (spawn/step, gate-intercept, nudge-inject, progress→segments).
  Keep it as-is for MCP wiring; P5 needs a **separate run-driver contract**.
- **P3's run drive hardcodes Claude.** `tasks/handlers/advance_run.rs::drive_run`
  builds `AgentCommand::new(cfg.agent_cmd /* "claude" */, ["-p", prompt], …)` via the
  P3.3a `agent_spawn` primitive. That's the one place a backend is chosen — the
  natural extraction point for the adapter.
- **The control channel already exists for Claude** (§5): the blocking PreToolUse
  `/hook/gate` (feature B) + the hard-block classifier + gemma4 backstop. That's the
  Claude adapter's gate mechanism; other backends need their own.
- **Capability ladder** (relay-engine §7): Hooks (Claude, rich/passive) · ACP
  (Zed, rich/sensei-drives) · Fallback (aider/Codex/CLI, coarse running/idle/done).

## The core contract (proposed — P5.1)

A new trait, e.g. `crates/senseid/src/relay_drivers/trait_def.rs::RunDriver`
(name TBD), the run-drive/supervise seam (distinct from the config `Assistant`
trait). Rough shape — to be firmed in P5.1:

```
trait RunDriver {
    fn id(&self) -> &str;                       // "claude" | "acp" | "fallback:aider"
    fn capability(&self) -> DriveCapability;    // Hooks | Acp | Fallback (feed richness)
    async fn drive_step(&self, ctx: &DriveCtx) -> DriveOutcome;   // one tick's work
    // gate/nudge/status normalized to the relay Execution→Segment→Item + control surface
    async fn gate(&self, …) -> GateDisposition;  // how a gate is raised/awaited for THIS backend
    async fn nudge(&self, …) -> Result<(), …>;   // steer (healthy) / unstick (stalled)
    fn observe(&self, …) -> DriveStatus;         // running/idle/done + progress (coarseness varies)
}
```
P3's `DriveOutcome`/`map_drive_outcome`/limit-detection/watchdog stay backend-agnostic
(they already are — they read an `AgentOutput`), so the adapter only owns *how the
step runs + how gates/nudges/status flow* for that backend.

## Chunks (per-chunk cadence = TDD → build/test → reviewer → commit `develop`)

### P5.1 — the RunDriver trait + Claude behind it (NO behavior change)  ·  Rust
- Extract the run-drive/gate/nudge/observe contract; implement it for **Claude Code**
  by moving today's `drive_run` (`claude -p` spawn + the existing `/hook/gate`
  control channel) behind the trait. `advance_run` selects the driver (default
  Claude) instead of hardcoding the command. Pure outcome-map + limit + watchdog
  unchanged. **Acceptance:** the P3 drive smoke still passes identically (a stub
  agent → FeatureStarted→FeatureDone), all P3 tests green, zero behavior change —
  the abstraction is proven with the one backend that works. Reviewer + (drive stays
  OFF by default). This is the safe, decision-light first chunk.
- **Autonomous.** Depends on nothing new.

### P5.2 — ACP adapter (Zed + adopters)  ·  Rust  ·  ⚠ OPEN DECISION (scope)
- The daemon hosts a run over the Apache-2.0 **Agent Client Protocol** (ACP) so an
  ACP-speaking assistant (Zed) is driven/observed richly with sensei in the driver
  seat. Needs: an ACP client/host in the daemon, session lifecycle, mapping ACP
  events → relay segments + gates → the control surface.
- **OPEN:** full drive-over-ACP vs a thinner **observe-first** (surface ACP session
  status/segments without full gate-drive) as P5.2a. ACP is a real external protocol
  integration (a lib/dep + a hosted session) — see decisions below.

### P5.3 — fallback adapter (coarse)  ·  Rust  ·  ⚠ OPEN DECISION (mechanism)
- aider / Codex / plain CLIs have no hooks/ACP → a coarse feed (running/idle/done +
  best-effort progress). **OPEN:** mechanism — MCP (sensei's MCP is already
  everywhere) vs file-watch vs poll vs OTLP ([decisions.md] lists all four).
  Recommend **MCP-based coarse status** first (lowest new infra; reuses the tool
  surface). Coarse gating = a time/– or checkpoint-based nudge, not live PreToolUse.

### P5.4 — orchestrator / control-surface normalization  ·  ⚠ OPEN DECISION (how much)
- The "orchestrator wrapper" (relay-engine §7 + the 2026-07-16 decision): one
  control surface (start·pause·gate·nudge·observe) each adapter satisfies, firing
  **separate planning + implementation cycles** so any backend is driven uniformly.
  The design says it "lands with P5" but flags it a **larger lift**. **OPEN:** build
  the full orchestrator now, or ship P5.1–P5.3 (the adapter layer) and defer the
  planning/impl-cycle orchestrator to P5.4-later once ≥2 backends exist to normalize.

## OPEN DECISIONS — need Jerry's plan/spec input BEFORE building past P5.1

1. **Sequencing / scope.** Start with **P5.1 only** (Claude behind the trait — the
   safe, no-behavior-change extraction that proves the abstraction), then reassess
   ACP/fallback? Or commit to the full P5.1→P5.4 arc now?
2. **ACP depth (P5.2).** Full drive-over-ACP, or observe-first (P5.2a) then drive?
   (Full ACP hosting is the biggest single lift in P5 — a real protocol + dep.)
3. **Fallback mechanism (P5.3).** MCP-coarse (recommended) vs file-watch / poll /
   OTLP for the first fallback backend?
4. **Orchestrator (P5.4).** Build the planning/impl-cycle orchestrator wrapper as
   part of P5, or defer it until the adapter layer (P5.1–P5.3) proves out with 2+
   backends?
5. **New dep(s).** ACP + any fallback transport may add external crates — raise each
   as its own decision when scoped (per [[feedback_external_dep_issue]]).

## Recommendation
Build **P5.1 now** (autonomous, decision-light, high-value: it de-risks the whole
phase by proving the adapter with the working Claude backend + zero behavior change),
and settle decisions 1–4 with Jerry before P5.2+. P5.1's reviewer + the unchanged P3
drive smoke are its gate.
