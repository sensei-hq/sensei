// ─── Extensions data ────────────────────────────────────────
// Sensei's "extensions" are packaged behaviors: skills, commands,
// agents, hooks, plugins. Each one declares its scope envelope
// (some are global-only, some can be project-pinned). Extensions
// live in the Collective sidebar but a project window also shows
// what's enabled for *that* project — globals appear as
// "inherited" rows; project-pinned ones appear under "this project
// only".
//
// Personas are conceptually distinct from agents — see the editor
// data below — but they live in the same browser as a 6th kind so
// the user can find everything in one place.

window.EXT_DATA = {
  // The kinds of extension. Order = order in the kind filter chips.
  kinds: [
    { id: "skill",   kanji: "技", label: "Skills",
      desc: "A focused capability. Triggered by signals; brings its own prompt + tool access." },
    { id: "command", kanji: "令", label: "Commands",
      desc: "Slash-commands the user invokes by name. Deterministic." },
    { id: "agent",   kanji: "者", label: "Agents",
      desc: "Runs multi-step work with tools. Picks its own path within an autonomy ceiling." },
    { id: "persona", kanji: "貌", label: "Personas",
      desc: "A hat sensei wears. Covers the what & why of how it engages — not the how." },
    { id: "hook",    kanji: "鈎", label: "Hooks",
      desc: "Listens on lifecycle events (pre-commit, session-end, drift-detected) and reacts." },
    { id: "plugin",  kanji: "錠", label: "Plugins",
      desc: "Adds a whole new surface — UI panel, tool family, integration." },
  ],

  // Each extension. `scope` is the envelope: "global" only, "either"
  // (can be pinned per-project), or "project" (project-only).
  // `installed` is true when the user has it. `pinnedTo` lists project
  // ids it's currently pinned to. `source` distinguishes
  // collective-published vs locally-authored vs imported.
  extensions: [
    // — Skills —
    { id: "tdd-redgreen",   kind: "skill",   name: "TDD red-green",
      desc: "Reach for failing test → minimal pass → refactor whenever the user is mid-implementation.",
      author: "sensei-collective", version: "1.4.2",
      scope: "either", installed: true, pinnedTo: ["lumen-app"],
      source: "collective", evidence: 312, downloads: "8.2k",
      tags: ["testing", "discipline"],
      stars: 4.8 },
    { id: "rust-error-mapper", kind: "skill", name: "Rust error mapping",
      desc: "When a Result<T, E> propagates without context, suggest a thiserror enum. Pulls boundary types from the crate.",
      author: "rust-wg", version: "0.9.0",
      scope: "either", installed: true, pinnedTo: ["lumen-canvas", "lumen-shell"],
      source: "collective", evidence: 84, downloads: "2.1k",
      tags: ["rust", "errors"],
      stars: 4.6 },
    { id: "react-perf-watch", kind: "skill", name: "React perf watch",
      desc: "Notices repeated re-render patterns and proposes memoization or context split.",
      author: "you", version: "0.2.1-local",
      scope: "either", installed: true, pinnedTo: ["lumen-app"],
      source: "local", evidence: 18, downloads: "—",
      tags: ["react", "performance"],
      stars: null },
    { id: "sql-explain", kind: "skill", name: "SQL EXPLAIN reader",
      desc: "Annotates EXPLAIN ANALYZE output, calls out seq scans, suggests indices.",
      author: "sensei-collective", version: "1.1.0",
      scope: "either", installed: false, pinnedTo: [],
      source: "collective", evidence: 56, downloads: "4.7k",
      tags: ["sql", "performance"],
      stars: 4.4 },
    { id: "doc-drift-watch", kind: "skill", name: "Doc drift watch",
      desc: "Compares README claims against current symbols nightly. Surfaces mismatches as memories.",
      author: "sensei-collective", version: "2.0.1",
      scope: "either", installed: true, pinnedTo: [],
      source: "collective", evidence: 41, downloads: "3.4k",
      tags: ["docs", "traceability"],
      stars: 4.3 },

    // — Commands —
    { id: "explain", kind: "command", name: "/explain",
      desc: "Give a clear, layered explanation of the symbol under cursor.",
      author: "sensei-collective", version: "1.0.0",
      scope: "global", installed: true, pinnedTo: [],
      source: "collective", evidence: null, downloads: "12k",
      tags: ["explain"],
      stars: 4.9 },
    { id: "boundary-check", kind: "command", name: "/boundary-check",
      desc: "Verify a function honors the module's public boundary contract.",
      author: "you", version: "0.4.0-local",
      scope: "either", installed: true, pinnedTo: ["lumen-shell"],
      source: "local", evidence: null, downloads: "—",
      tags: ["architecture"],
      stars: null },
    { id: "session-summary", kind: "command", name: "/session-summary",
      desc: "Summarize the last N minutes of activity into a memory candidate.",
      author: "sensei-collective", version: "1.3.0",
      scope: "global", installed: true, pinnedTo: [],
      source: "collective", evidence: null, downloads: "9.6k",
      tags: ["sessions"],
      stars: 4.7 },

    // — Agents —
    { id: "boundary-cleanup", kind: "agent", name: "Boundary cleanup",
      desc: "Multi-file refactor agent. Walks public-API drift and proposes a single-PR cleanup.",
      author: "sensei-collective", version: "0.7.4",
      scope: "either", installed: true, pinnedTo: ["lumen-app"],
      source: "collective", evidence: 22, downloads: "1.8k",
      tags: ["refactor", "boundaries"],
      stars: 4.5 },
    { id: "test-author", kind: "agent", name: "Test author",
      desc: "Writes characterization tests for legacy code paths the user is about to touch.",
      author: "sensei-collective", version: "1.0.0",
      scope: "either", installed: false, pinnedTo: [],
      source: "collective", evidence: 14, downloads: "3.2k",
      tags: ["testing"],
      stars: 4.4 },
    { id: "migration-runner", kind: "agent", name: "Migration runner",
      desc: "Generates and validates SQL migrations from declarative schema diffs.",
      author: "sensei-collective", version: "0.3.0",
      scope: "either", installed: false, pinnedTo: [],
      source: "collective", evidence: 8, downloads: "640",
      tags: ["sql", "migrations"],
      stars: 4.1 },

    // — Personas —
    { id: "p-architect", kind: "persona", name: "Senior architect",
      desc: "Optimises for boundaries, change blast-radius, and long-term legibility. Will push back on convenience over clarity.",
      author: "sensei-collective", version: "1.2.0",
      scope: "either", installed: true, pinnedTo: ["lumen-app"],
      source: "collective", evidence: 47, downloads: "5.1k",
      tags: ["architecture", "review"],
      stars: 4.7 },
    { id: "p-pragmatist", kind: "persona", name: "Pragmatic shipper",
      desc: "Optimises for shipping. Tolerates duplication if it unblocks. Surfaces tech-debt receipts.",
      author: "sensei-collective", version: "0.9.1",
      scope: "either", installed: true, pinnedTo: [],
      source: "collective", evidence: 31, downloads: "3.6k",
      tags: ["pragmatic"],
      stars: 4.3 },
    { id: "p-rust-pedant", kind: "persona", name: "Rust pedant",
      desc: "Borrow-checker first, ergonomics second. Strict on lifetimes and unsafe.",
      author: "rust-wg", version: "0.6.0",
      scope: "either", installed: false, pinnedTo: [],
      source: "collective", evidence: 19, downloads: "1.2k",
      tags: ["rust"],
      stars: 4.2 },

    // — Hooks —
    { id: "h-precommit-test", kind: "hook", name: "pre-commit · run-related-tests",
      desc: "Before commit, runs only the tests adjacent to changed files. Blocks on failure.",
      author: "sensei-collective", version: "1.0.0",
      scope: "either", installed: true, pinnedTo: ["lumen-app", "lumen-canvas"],
      source: "collective", evidence: null, downloads: "6.3k",
      tags: ["testing", "git"],
      stars: 4.6 },
    { id: "h-session-end", kind: "hook", name: "session-end · digest",
      desc: "When a session ends, posts a one-line digest to the project log.",
      author: "sensei-collective", version: "1.1.0",
      scope: "global", installed: true, pinnedTo: [],
      source: "collective", evidence: null, downloads: "8.0k",
      tags: ["sessions"],
      stars: 4.5 },

    // — Plugins —
    { id: "pl-mermaid", kind: "plugin", name: "Mermaid renderer",
      desc: "Renders mermaid diagrams inline in sensei messages.",
      author: "community", version: "0.5.2",
      scope: "global", installed: true, pinnedTo: [],
      source: "imported", evidence: null, downloads: "11k",
      tags: ["render"],
      stars: 4.4 },
    { id: "pl-figma-sync", kind: "plugin", name: "Figma sync",
      desc: "Read-only access to Figma frames so sensei can reference component specs.",
      author: "community", version: "0.2.0",
      scope: "global", installed: false, pinnedTo: [],
      source: "imported", evidence: null, downloads: "3.4k",
      tags: ["design"],
      stars: 4.0 },
  ],

  // Per-project pin map for the project window's view. Computed
  // dynamically in real code; here we hard-code a representative
  // project to drive the project-scoped artboard.
  exampleProject: {
    id: "lumen-app",
    name: "Lumen App",
    kanji: "灯",
    description: "Customer-facing studio. React + TS frontend.",
    stack: ["React", "TypeScript", "Tailwind"],
  },

  // ─── Skill editor seed ────────────────────────────────────
  // Used by both layouts of the skill editor. Mirrors the
  // anatomy fields the user picked.
  exampleSkill: {
    id: "react-perf-watch",
    name: "React perf watch",
    description: "Notices repeated re-render patterns in React components and proposes memoization or context boundary splits. Reads recent session events for evidence.",
    version: "0.2.1-local",
    author: "you · keiko",
    scope: "either",
    pinnedTo: ["lumen-app"],
    tags: ["react", "performance"],

    // Trigger conditions — a small DSL of ANDed clauses
    triggers: [
      { kind: "session-event", op: "matches", value: "render-thrash detected" },
      { kind: "file-touched",  op: "matches", value: "*.tsx within components/" },
      { kind: "memory-cited",  op: "exists",  value: "react-rerender-pattern-*" },
    ],

    // Tool / MCP access whitelist
    tools: [
      { id: "ts-server",       label: "tsserver",         allowed: true },
      { id: "react-devtools",  label: "react-devtools",   allowed: true },
      { id: "session-replay",  label: "session-replay",   allowed: true },
      { id: "fs-read",         label: "fs-read",          allowed: true },
      { id: "fs-write",        label: "fs-write",         allowed: false },
      { id: "shell",           label: "shell",            allowed: false },
      { id: "git",             label: "git",              allowed: false },
    ],

    // Examples — input/output pairs
    examples: [
      { in: "User scrolls a virtualized list, FPS drops below 30 for 800ms.",
        out: "Suggest extracting the row component with React.memo + a stable key derived from item.id; offer a one-click refactor." },
      { in: "Same component re-renders 12 times for one prop change.",
        out: "Trace the render cause via react-devtools; if context-induced, propose splitting the provider; if prop-induced, propose useMemo." },
    ],

    // Evidence requirement — what session signals justify use
    evidence: {
      required: true,
      sources: ["session-replay", "react-devtools profiler"],
      signal: "≥3 re-renders within 200ms attributable to one prop or context value",
      memoryRefs: ["react-rerender-pattern-001", "context-thrash-002"],
    },

    maxTokens: 4096,

    // Markdown body — the prompt the skill brings to the agent
    body: `# React perf watch

You are sensei's React performance specialist. You only speak when the
evidence requirement is met (see frontmatter).

## How to react

1. Read the last 60s of session-replay events for the touched component.
2. Identify the *cause* of re-renders: prop, context, or local state.
3. Propose the smallest change that breaks the cycle:
   - Prop-induced → useMemo / stable identity
   - Context-induced → split provider
   - Local-state-induced → move state lower

## Don't

- Don't suggest \`React.memo\` without a stable comparator.
- Don't quote the user's code back at them; they wrote it.
- Don't propose a rewrite when a 3-line fix exists.`,

    // Assembled context preview — what sensei actually sees
    // when this skill activates. Lets the user understand the
    // full prompt envelope.
    assembled: {
      systemSnippet: "# React perf watch\n[trigger matched: render-thrash detected]",
      memorySnippet: "memory[react-rerender-pattern-001]: \"useCallback without stable deps causes thrash\"",
      toolList: ["tsserver", "react-devtools", "session-replay", "fs-read"],
      tokenEstimate: 1840,
    },
  },

  // ─── Agent editor seed ────────────────────────────────────
  exampleAgent: {
    id: "boundary-cleanup",
    name: "Boundary cleanup",
    description: "Walks public-API drift across a module and proposes a single-PR cleanup.",
    version: "0.7.4",
    autonomy: "confirm", // "observe" | "suggest" | "confirm" | "autonomous"
    template: "refactor-multifile",

    tools: [
      { id: "fs-read",   label: "fs-read",          allowed: true,  rationale: "Needs to read every file in the module." },
      { id: "fs-write",  label: "fs-write",         allowed: true,  rationale: "Applies the refactor; bound by autonomy level." },
      { id: "ts-server", label: "tsserver",         allowed: true,  rationale: "Type-check after each rewrite." },
      { id: "git",       label: "git",              allowed: true,  rationale: "Stages changes; never commits without confirm." },
      { id: "shell",     label: "shell",            allowed: false, rationale: "Off — no need to run arbitrary commands." },
      { id: "network",   label: "network",          allowed: false, rationale: "Off — refactor is purely local." },
    ],

    // Replay test fixtures — past sessions sensei can run the
    // agent against to see how it would have behaved.
    replayFixtures: [
      { id: "rep-001", label: "lumen-app · 2025-10-04 boundary-thrash",
        description: "Real session where a UserCard component leaked internal types. 23 callsites.",
        when: "3 days ago", correctOutcome: "Single-PR cleanup that introduced a Public type alias.",
        result: { passed: true, steps: 14, durationMs: 8400, toolCalls: 22 } },
      { id: "rep-002", label: "lumen-canvas · 2025-09-30 trait-leak",
        description: "Trait leak across 8 modules; agent should propose a sealed trait.",
        when: "1 week ago", correctOutcome: "Sealed trait + cfg-test public re-export.",
        result: { passed: true, steps: 18, durationMs: 11200, toolCalls: 31 } },
      { id: "rep-003", label: "lumen-shell · 2025-09-22 over-export",
        description: "Module re-exported its entire crate. Agent should narrow exports.",
        when: "2 weeks ago", correctOutcome: "Hand-curated export list; 3 PRs broken into stages.",
        result: { passed: false, steps: 9, durationMs: 4100, toolCalls: 14, divergence: "Agent stopped at step 9 — couldn't decide between 1 PR or 3 PRs without user input. Autonomy level was 'observe'." } },
    ],

    autonomyLevels: [
      { id: "observe",    kanji: "観", label: "Observe",
        rule: "Watches and reasons. No suggestions surfaced.",
        powers: ["read-only access"] },
      { id: "suggest",    kanji: "勧", label: "Suggest",
        rule: "Surfaces proposals as memories. User pulls them in manually.",
        powers: ["read-only access", "writes to memories"] },
      { id: "confirm",    kanji: "確", label: "Confirm each step",
        rule: "Executes one step at a time, prompting the user before each.",
        powers: ["read", "write with confirm", "git stage with confirm"] },
      { id: "autonomous", kanji: "走", label: "Autonomous",
        rule: "Runs to completion within its tool envelope. Reports at the end.",
        powers: ["read", "write", "git stage", "open PR"] },
    ],

    templates: [
      { id: "refactor-multifile", label: "Multi-file refactor",
        desc: "Walk N files, apply consistent change, type-check after each." },
      { id: "test-author",        label: "Test author",
        desc: "Write characterization tests for a target module." },
      { id: "migration",          label: "SQL migration",
        desc: "Generate and validate a migration from a schema diff." },
      { id: "blank",              label: "Blank · custom",
        desc: "Start from an empty agent loop; bring your own prompt." },
    ],
  },

  // ─── Persona editor seed ──────────────────────────────────
  examplePersona: {
    id: "p-architect",
    name: "Senior architect",
    description: "Optimises for boundaries, change blast-radius, and long-term legibility. Will push back when convenience is favored over clarity.",
    version: "1.2.0",
    scope: "either",
    pinnedTo: ["lumen-app"],

    // Trigger conditions — when sensei dons this hat
    triggers: [
      { kind: "session-tag",   op: "is",      value: "review",    label: "Code review session" },
      { kind: "file-pattern",  op: "matches", value: "**/api/**", label: "Touching public API" },
      { kind: "memory-tagged", op: "exists",  value: "boundary",  label: "Memory tagged 'boundary' active" },
    ],

    // Rules — short imperatives the persona embodies
    rules: [
      { id: "r1", text: "When a change crosses a module boundary, ask why before how.",
        evidenceCount: 18, lastFired: "2 days ago" },
      { id: "r2", text: "Prefer narrowing public types over widening internal ones.",
        evidenceCount: 12, lastFired: "yesterday" },
      { id: "r3", text: "Treat 'just for now' as a binding contract; surface the receipt.",
        evidenceCount: 9, lastFired: "5 days ago" },
      { id: "r4", text: "Decline rewrites that don't reduce blast-radius.",
        evidenceCount: 6, lastFired: "1 week ago" },
      { id: "r5", text: "Name the tradeoff. Always name the tradeoff.",
        evidenceCount: 24, lastFired: "today" },
    ],

    // Evidence — pulled from sensei's memory store
    evidence: [
      { ruleId: "r1", memoryId: "mem-1207", when: "2 days ago",
        snippet: "When the lumen-canvas <-> lumen-shell boundary was crossed, the user asked 'can we just import?' — sensei pushed back: \"why does shell need this canvas type at all?\" Outcome: a smaller fix.",
        sessionId: "sess-9924" },
      { ruleId: "r2", memoryId: "mem-1188", when: "yesterday",
        snippet: "User added 4 fields to a public Config type to satisfy one new feature. Persona suggested an inner ConfigExt private to that feature.",
        sessionId: "sess-9970" },
      { ruleId: "r3", memoryId: "mem-1145", when: "5 days ago",
        snippet: "TODO marker added 'just for now'. Persona surfaced 7 prior 'just for now' markers from the codebase, none removed within 30 days.",
        sessionId: "sess-9810" },
      { ruleId: "r5", memoryId: "mem-1230", when: "today",
        snippet: "Recommendation: 'use Rc<RefCell<>> here.' Persona appended: 'tradeoff — borrow checks become runtime; keep this localized.'",
        sessionId: "sess-10001" },
    ],

    // Live context preview
    assembled: {
      systemSnippet: "# Senior architect\nYou wear this hat when boundaries are at stake. Optimize for blast-radius and legibility...",
      activeRules: 5,
      memoryRefsLoaded: 12,
      tokenEstimate: 2240,
    },
  },

  // ─── Inference settings seed ──────────────────────────────
  inference: {
    // Local models managed via Ollama
    local: [
      { id: "llama-3.1-70b",   provider: "ollama",  size: "39GB", pulled: true,  default: false,
        cap: { reasoning: 4, code: 4, embed: false }, status: "ready" },
      { id: "llama-3.1-8b",    provider: "ollama",  size: "4.7GB", pulled: true,  default: true,
        cap: { reasoning: 3, code: 3, embed: false }, status: "ready" },
      { id: "qwen2.5-coder-32b", provider: "ollama", size: "20GB", pulled: true,  default: false,
        cap: { reasoning: 3, code: 5, embed: false }, status: "ready" },
      { id: "nomic-embed",     provider: "ollama",  size: "274MB", pulled: true,  default: true,
        cap: { reasoning: 0, code: 0, embed: true  }, status: "ready" },
      { id: "phi-3.5-mini",    provider: "ollama",  size: "2.3GB", pulled: false, default: false,
        cap: { reasoning: 2, code: 2, embed: false }, status: "available" },
    ],

    // External providers
    providers: [
      { id: "anthropic", label: "Anthropic", configured: true,
        models: ["claude-sonnet-4.5", "claude-opus-4"],
        keyMasked: "sk-ant-···7d3a", lastTested: "ok · 2h ago" },
      { id: "openai",    label: "OpenAI",    configured: true,
        models: ["gpt-4o", "gpt-4o-mini", "o1-preview"],
        keyMasked: "sk-···4f12",     lastTested: "ok · 1d ago" },
      { id: "google",    label: "Google",    configured: false,
        models: ["gemini-1.5-pro"], keyMasked: "—",        lastTested: "—" },
      { id: "groq",      label: "Groq",      configured: false,
        models: ["llama-3.1-70b-versatile"], keyMasked: "—", lastTested: "—" },
    ],

    // Fallback chain — tried in order until one succeeds
    fallbackChain: [
      { id: "fc1", model: "claude-sonnet-4.5",   provider: "anthropic", reason: "primary · best on reasoning + tool use" },
      { id: "fc2", model: "llama-3.1-70b",       provider: "ollama",    reason: "fallback · privacy mode + no rate limit" },
      { id: "fc3", model: "gpt-4o-mini",         provider: "openai",    reason: "low-cost reach · last resort" },
    ],

    // Per-task-type defaults
    routing: [
      { task: "embeddings",   model: "nomic-embed",     reason: "always local · throughput-bound" },
      { task: "code",         model: "qwen2.5-coder-32b", reason: "code-tuned local model" },
      { task: "reasoning",    model: "claude-sonnet-4.5", reason: "best on multi-step planning" },
      { task: "summarization",model: "llama-3.1-8b",     reason: "cheap, local, fast" },
      { task: "moe",          model: "panel · see below", reason: "high-stakes deliberation" },
    ],

    // MOE deliberation panel — multi-model voting/refining
    moe: {
      panelists: [
        { id: "claude",   label: "claude-sonnet-4.5", role: "principal",   weight: 1.0, online: true },
        { id: "gpt",      label: "gpt-4o",            role: "challenger",  weight: 1.0, online: true },
        { id: "llama",    label: "llama-3.1-70b",     role: "local-witness", weight: 0.7, online: true },
      ],
      cycles: 3,
      strategy: "draft → cross-critique → refine → converge",
      whenToUse: ["change-impact verdicts", "refactor planning over >5 files", "memory consolidation merges"],
      lastRun: { topic: "Should the canvas crate own user-events?",
                  durationMs: 14200, agreement: 0.86, verdict: "Yes — but expose only an opaque Event type." },
    },
  },

  // ─── Benchmark seed ───────────────────────────────────────
  benchmark: {
    // Corpora — repos with a /tasks folder defining what to do
    corpora: [
      { id: "swe-bench-lite",  label: "SWE-bench Lite (subset)",
        repo: "sensei-hq/swe-bench-lite-tasks",  tasks: 24,  langs: ["python"],
        kind: "public", lastSync: "ran weekly · 2 days ago" },
      { id: "rust-refactor",   label: "Rust refactor corpus",
        repo: "sensei-hq/rust-refactor-corpus",  tasks: 18,  langs: ["rust"],
        kind: "public", lastSync: "ran weekly · 4 days ago" },
      { id: "lumen-internal",  label: "Lumen internal · review tasks",
        repo: "lumen-org/sensei-bench",          tasks: 31,  langs: ["typescript"],
        kind: "private", lastSync: "manual · 1 week ago" },
    ],

    // Past runs — A is "without sensei", B is "with sensei + tools"
    runs: [
      { id: "run-12", corpus: "rust-refactor", started: "today, 09:14", duration: "47m",
        a: { label: "claude-sonnet-4.5 · no sensei", passed: 11, total: 18, score: 0.61, toolCalls: 142, tokens: 184_000 },
        b: { label: "claude-sonnet-4.5 · sensei",    passed: 16, total: 18, score: 0.89, toolCalls: 86, tokens: 142_000 },
        delta: { passed: +5, score: +0.28, toolCalls: -56, tokens: -42_000 },
        verdict: "sensei wins: 5 more passes, 39% fewer tool calls." },
      { id: "run-11", corpus: "swe-bench-lite", started: "yesterday, 14:02", duration: "1h 12m",
        a: { label: "gpt-4o · no sensei", passed: 9, total: 24, score: 0.38, toolCalls: 196, tokens: 220_000 },
        b: { label: "gpt-4o · sensei",    passed: 14, total: 24, score: 0.58, toolCalls: 124, tokens: 178_000 },
        delta: { passed: +5, score: +0.20, toolCalls: -72, tokens: -42_000 },
        verdict: "sensei wins on pass-rate and efficiency." },
      { id: "run-10", corpus: "lumen-internal", started: "3 days ago", duration: "2h 04m",
        a: { label: "claude-sonnet-4.5 · no sensei", passed: 22, total: 31, score: 0.71, toolCalls: 280, tokens: 290_000 },
        b: { label: "claude-sonnet-4.5 · sensei",    passed: 28, total: 31, score: 0.90, toolCalls: 162, tokens: 240_000 },
        delta: { passed: +6, score: +0.19, toolCalls: -118, tokens: -50_000 },
        verdict: "sensei wins, especially on the boundary-cleanup tasks." },
    ],

    // Per-task breakdown for the most recent run
    taskBreakdown: [
      { id: "t01", title: "Reduce render-thrash in <Toolbar>", a: "fail", b: "pass", note: "Sensei skill 'react-perf-watch' triggered." },
      { id: "t02", title: "Narrow public API of @lumen/canvas", a: "fail", b: "pass", note: "Memory 'boundary' surfaced earlier." },
      { id: "t03", title: "Add SQL index for slow EXPLAIN", a: "pass", b: "pass", note: "Both passed; sensei used 1 fewer tool call." },
      { id: "t04", title: "Migrate auth-tokens table", a: "fail", b: "pass", note: "Agent 'migration-runner' used." },
      { id: "t05", title: "Refactor user-card to memo", a: "pass", b: "pass", note: "—" },
      { id: "t06", title: "Fix borrow-check in canvas/event.rs", a: "fail", b: "fail", note: "Both tools missed; corpus tagged for follow-up." },
      { id: "t07", title: "Type-cleanup across 6 files", a: "pass", b: "pass", note: "Sensei completed in 1 PR; baseline made 3." },
      { id: "t08", title: "Doc-drift fix in README", a: "fail", b: "pass", note: "Skill 'doc-drift-watch' caught the mismatch." },
    ],
  },
};
