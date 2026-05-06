// Sensei — collective intelligence + drift + impact + consolidation data.
// Mirrors what the spec promises:
//   inference.recommendations  → impact reports + verdict
//   inference.drift_items      → traceability
//   inference.insights         → outbound shares
//   inference.reasoning_traces → MOE panel reasoning
//   sensei.memory_links        → consolidation candidates
//   sensei.nodes               → doc + code symbols

window.UPGRADES = (function () {

  // ─── Incoming upgrades from the collective KB ───────────────
  // What other senseis have discovered, packaged, and surfaced to you.
  // Each is a candidate; the user decides whether to install.
  const incoming = [
    {
      id: "u-agent-rust-axum-handler",
      kind: "agent",
      glyph: "作",
      name: "rust-axum-handler.agent.md",
      title: "axum-handler agent",
      summary: "Steps through the canonical sub-router → handler → Result envelope flow whenever you write a new HTTP handler.",
      why: "Detected in 11 of your last 20 lumen-cloud sessions. The collective sees +14% FTR on average when this agent runs first.",
      stack: ["rust", "axum"],
      relevantProjects: ["lumen", "tabi"],
      contributors: 47,            // # of senseis whose data fed this
      adoptions: 312,              // # of installs across the network
      avgFtrLift: 0.14,
      maturity: "battle-tested",   // proposed | active | battle-tested
      received: "2 days ago",
      sourceModel: "collective · q-2026-04",
      conflicts: [],
      preview: [
        "1. Run `cargo check` on the target file",
        "2. Locate the closest `Router::new()` and identify the sub-router",
        "3. Draft handler signature returning Result<Json<T>, ApiError>",
        "4. Wire route into the sub-router with rate-limit + auth layers"
      ]
    },
    {
      id: "u-skill-retry-backoff",
      kind: "skill",
      glyph: "技",
      name: "retry-with-backoff.skill.md",
      title: "Retry-with-backoff skill",
      summary: "Drop-in helper for HTTP retry with exponential backoff + jitter. Sensei references this whenever you write a fetch call against a flaky service.",
      why: "Your `p-retry-backoff` pattern is on the cusp of promotion (4 occurrences, copy-pasted). The collective has the canonical version — installing avoids divergence.",
      stack: ["typescript", "rust"],
      relevantProjects: ["lumen", "tabi"],
      contributors: 91,
      adoptions: 1240,
      avgFtrLift: 0.09,
      maturity: "battle-tested",
      received: "5 days ago",
      sourceModel: "collective · q-2026-03",
      conflicts: [],
      preview: [
        "Wraps fetch / reqwest with exp(2^n) backoff + ±20% jitter",
        "Stops at 5 retries or first non-retryable status",
        "Emits structured logs sensei can correlate"
      ]
    },
    {
      id: "u-command-check-handlers",
      kind: "command",
      glyph: "令",
      name: "/lumen-check-handlers",
      title: "/lumen-check-handlers",
      summary: "Lints all handlers under `src/handlers/*` for the Result-envelope memory + duplicate error-handling anti-pattern.",
      why: "You have 12 inline error-conversion sites flagged by `p-duplicate-error-handling`. This command runs the lint + suggests refactors.",
      stack: ["rust"],
      relevantProjects: ["lumen"],
      contributors: 3,             // smaller — niche
      adoptions: 28,
      avgFtrLift: 0.07,
      maturity: "active",
      received: "yesterday",
      sourceModel: "collective · q-2026-04",
      conflicts: [
        { kind: "memory", id: "m-result-api", note: "Existing memory governs the same surface — command will reinforce it." }
      ],
      preview: [
        "Greps src/handlers/* for `Err(ApiError::from(",
        "Reports each call-site grouped by error variant",
        "Suggests a shared `From<X> for ApiError` impl per variant"
      ]
    },
    {
      id: "u-lint-no-mock-db",
      kind: "lint",
      glyph: "禁",
      name: "no-mock-db.lint",
      title: "Lint: no mocked DB in integration tests",
      summary: "AST-level check that flags `MockPool` / `MockClient` constructions inside `tests/integration/*`.",
      why: "You learned this the hard way (m-no-mock-db, strength 5). The lint formalizes it so it can never regress silently.",
      stack: ["rust", "sqlx"],
      relevantProjects: ["lumen"],
      contributors: 22,
      adoptions: 184,
      avgFtrLift: 0.05,
      maturity: "battle-tested",
      received: "1 week ago",
      sourceModel: "collective · q-2026-03",
      conflicts: [],
      preview: [
        "Walks tests/integration/* AST",
        "Flags `MockPool::new`, `mock_db!`, `mockall::mock!` in test scope",
        "Pairs with the existing m-no-mock-db memory; failures cite it"
      ]
    },
    {
      id: "u-skill-svelte5-state",
      kind: "skill",
      glyph: "技",
      name: "svelte5-state.skill.md",
      title: "Svelte 5 — $state migration",
      summary: "Walks any `let x = …` declaration in a Svelte component and either migrates to `$state(x)` or explains why it's intentionally not reactive.",
      why: "Your `m-svelte5-state` memory has been violated 3× in the past week. Strength is 2, which is below sensei's surfacing threshold.",
      stack: ["svelte@5"],
      relevantProjects: ["koto"],
      contributors: 14,
      adoptions: 76,
      avgFtrLift: 0.11,
      maturity: "active",
      received: "3 hours ago",
      sourceModel: "collective · q-2026-04",
      conflicts: [],
      preview: [
        "Identifies top-level `let` declarations in .svelte files",
        "Distinguishes prop / local / shared state",
        "Rewrites to `$state(...)` and updates references"
      ]
    }
  ];

  // ─── Outbound — what your sensei has shared / will share ───
  // Each insight links to a source record (correction / pattern / etc.)
  // and gets bundled into a batch that ships on a cadence.
  const sharingMode = "review";   // "auto" | "review" | "off"
  const cadence = "weekly";

  // Items queued for the *next* batch (the user reviews before sending)
  const nextBatch = {
    id: "b-2026-04-29",
    scheduledFor: "Wed · 29 Apr",
    insights: [
      {
        id: "i-pattern-adapter",
        category: "pattern",
        glyph: "紋",
        title: "Adapter wraps third-party SDK behind a trait",
        summary: "Seen in 7 places across lumen-cloud · FTR +18% where applied · 0 violations in 6 weeks.",
        anonymizationNote: "Project name and exact paths stripped. Pattern shape + stack + delta are kept.",
        sourceKind: "memory",
        sourceId: "m-auth-adapter",
        evidence: 7,
        confidence: 0.92
      },
      {
        id: "i-correction-svelte-state",
        category: "correction",
        glyph: "直",
        title: "Recurring correction · `let` → `$state(...)` in Svelte 5",
        summary: "6 corrections in 7 days · same shape every time · confirms the legacy-reactivity drift gap.",
        anonymizationNote: "Stripped to the AST pattern. No file paths, no project tag.",
        sourceKind: "correction",
        sourceId: "c-svelte-state",
        evidence: 6,
        confidence: 0.88
      },
      {
        id: "i-anti-duplicate-err",
        category: "anti_pattern",
        glyph: "禁",
        title: "Anti-pattern · copy-pasted Err(ApiError::from(...))",
        summary: "12 occurrences across handlers — already two divergent maps for the same error variant. Diverges fast.",
        anonymizationNote: "Module names abstracted; only error-variant shape kept.",
        sourceKind: "pattern",
        sourceId: "p-duplicate-error-handling",
        evidence: 12,
        confidence: 0.88
      },
      {
        id: "i-ftr-rust-axum",
        category: "ftr",
        glyph: "果",
        title: "Stack-level FTR · rust + axum + sub-router pattern",
        summary: "Sessions using the domain-subrouter pattern report 0.81 FTR vs 0.64 baseline on flat routers. n=14 sessions.",
        anonymizationNote: "Aggregate ratio only. No session IDs, no project tag.",
        sourceKind: "stack-stat",
        sourceId: "ss-rust-axum",
        evidence: 14,
        confidence: 0.83
      }
    ]
  };

  // History — what shipped previously
  const sharingHistory = [
    { id: "b-2026-04-22", date: "22 Apr", insights: 5,
      categories: ["pattern", "correction", "stack"], helpedUsers: 41 },
    { id: "b-2026-04-15", date: "15 Apr", insights: 7,
      categories: ["pattern", "anti_pattern", "ftr", "tool"], helpedUsers: 63 },
    { id: "b-2026-04-08", date: "8 Apr",  insights: 3,
      categories: ["correction", "skill"], helpedUsers: 22 },
    { id: "b-2026-04-01", date: "1 Apr",  insights: 6,
      categories: ["pattern", "model", "ftr"], helpedUsers: 58 },
    { id: "b-2026-03-25", date: "25 Mar", insights: 4,
      categories: ["pattern", "anti_pattern"], helpedUsers: 37 }
  ];

  const contribution = {
    insightsShared: 184,
    usersHelped: 612,
    bestCategory: "pattern",
    bestCategoryCount: 71,
    streak: 12,    // consecutive weekly batches contributed
    rank: "top 9%"
  };

  // ─── Document traceability — drift items ────────────────────
  // Per-project rollup, then per-doc detail with each reference.
  const trace = {
    projectRollup: [
      { id: "lumen",  kanji: "月", name: "lumen-cloud", docs: 14, links: 142,
        current: 118, drifted: 18, broken: 6, healthPct: 0.83 },
      { id: "koto",   kanji: "琴", name: "koto-editor", docs: 9,  links: 67,
        current: 61,  drifted: 4,  broken: 2, healthPct: 0.91 },
      { id: "sensei", kanji: "先", name: "sensei",      docs: 22, links: 218,
        current: 198, drifted: 14, broken: 6, healthPct: 0.91 },
      { id: "tabi",   kanji: "旅", name: "tabi-sdk",    docs: 7,  links: 38,
        current: 28,  drifted: 6,  broken: 4, healthPct: 0.74 }
    ],
    docs: [
      {
        id: "d-auth-readme",
        project: "lumen",
        path: "src/auth/README.md",
        title: "auth/README.md",
        links: 24, current: 17, drifted: 4, broken: 3,
        lastChecked: "today",
        lastModified: "47 days ago",
        references: [
          { id: "ref-1", lineRef: "L18", quote: "calls `refresh_token(session_id, opts)`",
            target: { symbol: "refresh_token", path: "src/auth/refresh.rs:88" },
            status: "drifted",
            expected: "fn refresh_token(session_id: SessionId, opts: RefreshOpts) -> Result<Token, AuthError>",
            actual:   "fn refresh_token(session_id: SessionId, opts: RefreshOpts, audit: &AuditLog) -> Result<Token, AuthError>",
            diff: "+ audit: &AuditLog",
            reason: "Signature gained `audit: &AuditLog` in commit a91f2e (32d ago)."
          },
          { id: "ref-2", lineRef: "L42", quote: "see `src/auth/session.ts:142`",
            target: { symbol: "createSession", path: "src/auth/session.ts:142" },
            status: "broken",
            expected: "fn createSession(...)",
            actual:   "(symbol no longer exists)",
            diff: "− createSession",
            reason: "Removed in the Rust migration; replaced by `Session::start`."
          },
          { id: "ref-3", lineRef: "L67", quote: "the `inFlightMutex` ensures...",
            target: { symbol: "inFlightMutex", path: "src/auth/refresh.rs:14" },
            status: "current",
            expected: "static inFlightMutex: Mutex<HashMap<SessionId, ...>>",
            actual:   "static inFlightMutex: Mutex<HashMap<SessionId, ...>>",
            diff: "",
            reason: ""
          },
          { id: "ref-4", lineRef: "L89", quote: "rate-limited via `auth_rl_tower`",
            target: { symbol: "auth_rl_tower", path: "src/middleware/rate.rs:31" },
            status: "drifted",
            expected: "Tower<AuthBucket, 60/min>",
            actual:   "Tower<AuthBucket, 30/min>",
            diff: "60/min → 30/min",
            reason: "Limit tightened in commit 4c81b0 (12d ago)."
          }
        ]
      },
      {
        id: "d-handlers-style",
        project: "lumen",
        path: "docs/handlers-style.md",
        title: "docs/handlers-style.md",
        links: 18, current: 16, drifted: 2, broken: 0,
        lastChecked: "today",
        lastModified: "9 days ago",
        references: []
      },
      {
        id: "d-sync-crdt",
        project: "koto",
        path: "src/sync/crdt.md",
        title: "sync/crdt.md",
        links: 11, current: 11, drifted: 0, broken: 0,
        lastChecked: "today",
        lastModified: "2 days ago",
        references: []
      },
      {
        id: "d-tabi-quickstart",
        project: "tabi",
        path: "docs/quickstart.md",
        title: "docs/quickstart.md",
        links: 12, current: 6, drifted: 4, broken: 2,
        lastChecked: "today",
        lastModified: "94 days ago",
        references: []
      }
    ]
  };

  // ─── Change-impact reports for accepted recommendations ─────
  // Each closes the loop on one inference.recommendations row.
  const impactReports = [
    {
      id: "ir-promote-adapter",
      recId: "r-promote-adapter",
      title: "Promoted adapter pattern to project rule",
      project: "lumen",
      acted: "16 Apr",
      measured: "26 Apr",
      window: "10 days · 31 sessions",
      verdict: "positive",
      baselineFtr: 0.64,
      currentFtr:  0.78,
      ftrDelta:    +0.14,
      baselineCorrections: 2.4,
      currentCorrections:  1.1,
      correctionsDelta:    -1.3,
      toolUsageDelta: { "auth_adapter::*": +18, "raw http::*": -22 },
      avgSessionDelta: -7,    // minutes
      moeReasoning: {
        headline: "Strong positive impact across all measured axes.",
        body: "The adapter pattern reduced inline auth boilerplate, which is where most lumen-cloud corrections lived. Sessions touching auth/* dropped from 2.4 to 1.1 average corrections, and FTR moved from 0.64 to 0.78. Tool usage shifted as expected — auth_adapter calls up, raw http calls down. No regressions in unrelated modules. Recommend keeping; consider transferring to tabi-sdk where the same anti-pattern was flagged.",
        models: [
          { name: "claude-haiku", verdict: "positive", note: "FTR delta exceeds 2σ; sustained over 10 days." },
          { name: "qwen2.5-7b",   verdict: "positive", note: "Tool-usage shift matches the rule's intent." },
          { name: "kimi-k2-mini", verdict: "neutral",  note: "Sample size adequate but auth-only; transferable signal weaker." }
        ],
        consensus: "2 positive · 1 neutral · 0 negative"
      }
    },
    {
      id: "ir-archive-caching",
      recId: "r-archive-caching",
      title: "Archived Cache-Control memory",
      project: "lumen",
      acted: "12 Apr",
      measured: "24 Apr",
      window: "12 days · 22 sessions",
      verdict: "neutral",
      baselineFtr: 0.79,
      currentFtr:  0.78,
      ftrDelta:    -0.01,
      baselineCorrections: 0.4,
      currentCorrections:  0.3,
      correctionsDelta:    -0.1,
      toolUsageDelta: {},
      avgSessionDelta: 0,
      moeReasoning: {
        headline: "No measurable effect — safe archive.",
        body: "The memory was already inactive (strength 0). Archiving it removed noise without affecting any behaviour. Continue.",
        models: [
          { name: "claude-haiku", verdict: "neutral", note: "Within noise floor; sample bound." },
          { name: "qwen2.5-7b",   verdict: "neutral", note: "Same." },
          { name: "kimi-k2-mini", verdict: "neutral", note: "Same." }
        ],
        consensus: "3 neutral"
      }
    },
    {
      id: "ir-svelte-strength-bump",
      recId: "r-supersede-svelte",
      title: "Reinforced m-svelte5-state to strength 4",
      project: "koto",
      acted: "20 Apr",
      measured: "27 Apr",
      window: "7 days · 14 sessions",
      verdict: "negative",
      baselineFtr: 0.71,
      currentFtr:  0.62,
      ftrDelta:    -0.09,
      baselineCorrections: 1.2,
      currentCorrections:  1.9,
      correctionsDelta:    +0.7,
      toolUsageDelta: { "svelte_lsp::diagnostics": +34, "edit::svelte": -8 },
      avgSessionDelta: +12,
      moeReasoning: {
        headline: "Negative — the surfacing threshold was lowered too aggressively.",
        body: "Bumping strength from 2 to 4 caused sensei to inject the rule into context for every Svelte session, including non-component files where `let` reactivity is intentional. The assistant began over-correcting `let` declarations in plain `.ts` modules, which the user then had to revert. Net: more corrections, lower FTR, longer sessions. Recommend reverting to strength 2 and instead writing a tighter scope (only .svelte files at top level).",
        models: [
          { name: "claude-haiku", verdict: "negative", note: "+0.7 corrections, FTR -9pp; clear regression." },
          { name: "qwen2.5-7b",   verdict: "negative", note: "Tool-usage shift shows over-application; non-target files affected." },
          { name: "kimi-k2-mini", verdict: "negative", note: "Same direction; recommend revising scope, not strength." }
        ],
        consensus: "3 negative",
        suggestedRevision: "Keep strength 4 BUT scope to: file-glob `**/*.svelte`, top-level declarations only."
      }
    }
  ];

  // ─── Memory consolidation candidates ────────────────────────
  // Each: a parent (proposed merged memory) and N source memories.
  const consolidations = [
    {
      id: "cs-error-handling",
      title: "Error envelope across handlers",
      reason: "Three memories all govern handler-level error shape with overlapping evidence. Merge keeps a single canonical statement.",
      sourceIds: ["m-result-api", "m-auth-adapter"],   // demo: pretend they overlap
      sources: [
        {
          id: "m-result-api",
          what: "API handlers return Result<Json<T>, ApiError>.",
          because: "Unified error envelope — the frontend relies on ApiError.code for retry logic.",
          strength: 4,
          evidence: ["s-2601", "s-2733"],
          violated: 2,
          scope: { level: "project", project: "lumen", modules: ["handlers/*"] }
        },
        {
          id: "m-handlers-error-variant",
          what: "Map domain errors via a single `From<X> for ApiError` impl per variant.",
          because: "Twelve copy-pasted error conversions diverged within a month — two map the same NotFound differently.",
          strength: 3,
          evidence: ["s-2890", "s-2901"],
          violated: 1,
          scope: { level: "project", project: "lumen", modules: ["handlers/*", "errors/*"] }
        }
      ],
      proposed: {
        what: "API handlers return Result<Json<T>, ApiError>; map domain errors via a single From<X> impl per variant.",
        because: "Unified envelope is what the frontend retries against (ApiError.code), and the From-per-variant rule prevents the divergent maps that caused two Q1 incidents.",
        strength: 4,
        scope: { level: "project", project: "lumen", modules: ["handlers/*", "errors/*"] },
        evidence: ["s-2601", "s-2733", "s-2890", "s-2901"],
        violations: 3
      }
    },
    {
      id: "cs-trailing-text",
      title: "Response style — terse, no trailing text",
      reason: "Two memories say almost the same thing; one is a strict subset of the other's evidence.",
      sourceIds: ["m-no-trailing-summaries", "m-no-postscripts"],
      sources: [
        {
          id: "m-no-trailing-summaries",
          what: "Terse responses. No trailing summaries.",
          because: "User preference, reinforced 12 times.",
          strength: 5,
          evidence: ["s-1820", "s-1877", "s-2001", "s-2204"],
          violated: 0,
          scope: { level: "global" }
        },
        {
          id: "m-no-postscripts",
          what: "Skip 'Let me know if…' postscripts.",
          because: "Same as the trailing-summary preference; user always deletes them.",
          strength: 4,
          evidence: ["s-2380", "s-2412"],
          violated: 0,
          scope: { level: "global" }
        }
      ],
      proposed: {
        what: "Terse responses. No trailing summaries, no 'let me know if…' postscripts.",
        because: "Two memories with identical user behavior — the trailing summary and the postscript both got deleted in every session that triggered them. Combined evidence: 6 sessions across 9 months.",
        strength: 5,
        scope: { level: "global" },
        evidence: ["s-1820", "s-1877", "s-2001", "s-2204", "s-2380", "s-2412"],
        violations: 0
      }
    },
    {
      id: "cs-svelte-state",
      title: "Svelte 5 — $state migration",
      reason: "Memory + recurring correction tell the same story. Merging promotes the memory to a settled rule.",
      sourceIds: ["m-svelte5-state", "c-svelte-state"],
      sources: [
        {
          id: "m-svelte5-state",
          what: "Use $state not `let` for reactivity in Svelte 5.",
          because: "Legacy `let` reactivity silently stops updating across component boundaries in Svelte 5.",
          strength: 2,
          evidence: ["s-2780"],
          violated: 3,
          scope: { level: "stack", stack: ["svelte@5"] }
        },
        {
          id: "c-svelte-state",
          what: "(correction) `let x = …` → `let x = $state(…)`",
          because: "Same surface as the memory above; this is the recurring correction the user keeps applying.",
          strength: 3,
          evidence: ["s-2891", "s-2895", "s-2901", "s-2904", "s-2905", "s-2907"],
          violated: 0,
          scope: { level: "stack", stack: ["svelte@5"] }
        }
      ],
      proposed: {
        what: "Svelte 5 components: `let x = …` is non-reactive; use `$state(…)` for any value the template reads.",
        because: "The memory and the recurring correction govern the same rewrite. Combined evidence (7 sessions) is enough to settle.",
        strength: 4,    // max(2, 3) + 1 for the merge
        scope: { level: "stack", stack: ["svelte@5"], filePatterns: ["**/*.svelte"] },
        evidence: ["s-2780", "s-2891", "s-2895", "s-2901", "s-2904", "s-2905", "s-2907"],
        violations: 3
      }
    }
  ];

  return {
    incoming, nextBatch, sharingHistory, contribution,
    sharingMode, cadence,
    trace, impactReports, consolidations
  };
})();
