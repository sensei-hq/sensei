// Learnings — data
//
// Mirrors the memory anatomy spec: what / because / scope / strength / references / source / lifecycle.
// Patterns = code signals detected by sensei. Corrections = recurring fixes.
// Recommendations = suggested actions the user can take.
//
// Everything lives under window.LEARNINGS.

window.LEARNINGS = (function () {

  // ── Projects this observatory knows about ────────────────
  const PROJ = {
    "lumen":   { kanji: "月", name: "lumen-cloud",   client: "Lumen"   },
    "koto":    { kanji: "琴", name: "koto-editor",   client: "Koto"    },
    "ginkgo":  { kanji: "銀", name: "ginkgo",        client: "Personal" },
    "sensei":  { kanji: "先", name: "sensei",        client: "Sensei"  },
    "tabi":    { kanji: "旅", name: "tabi-sdk",      client: "Tabi"    }
  };

  // ── Memories ─────────────────────────────────────────────
  // Each memory: what, because, scope, strength (1–5), state, source,
  // references {good_example, bad_example, pattern, evidence, related, doc},
  // lifecycle events (learned / reinforced / challenged / archived).
  const memories = [
    {
      id: "m-auth-adapter",
      what: "Use the adapter pattern for auth integrations.",
      because: "Inline auth in handlers diverges — sync.ts forgot audit logging, caused a silent 3-day incident in Q1.",
      scope: { level: "project", project: "lumen", modules: ["auth/*"], taskTypes: ["feat", "refactor"] },
      strength: 4,
      state: "reinforced",
      source: "correction",
      category: "correctness",
      references: {
        good_example: "src/middleware/auth_adapter.rs:14",
        bad_example:  "src/handlers/auth.ts:42",
        pattern:      "adapter",
        evidence:     ["s-2891", "s-2895", "s-2901"],
        related:      ["m-audit-log"]
      },
      learned: "2026-04-15",
      reinforced: 2,
      violated: 0,
      lastRelevant: "2 days ago"
    },
    {
      id: "m-no-mock-db",
      what: "Don't mock the database in integration tests.",
      because: "Q1 2026: mocked tests passed, prod migration failed. 3 days of debugging. The mock diverged from PostgreSQL nullable-FK behavior.",
      scope: { level: "project", project: "lumen", modules: ["tests/integration/*"], taskTypes: ["test"] },
      strength: 5,
      state: "battle-tested",
      source: "correction",
      category: "correctness",
      references: {
        good_example: "tests/integration/auth_flow.rs:8",
        bad_example:  "tests/integration/legacy_billing.rs:112",
        evidence:     ["s-2410", "s-2411"]
      },
      learned: "2026-01-22",
      reinforced: 4,
      violated: 0,
      lastRelevant: "yesterday"
    },
    {
      id: "m-inflight-mutex",
      what: "Always use inFlightMutex for concurrent token refresh.",
      because: "Race condition without it — two parallel refreshes burn the rotation window and lock users out for 30s (s-2895).",
      scope: { level: "module", project: "lumen", modules: ["auth/refresh.ts"], taskTypes: ["fix", "feat"] },
      strength: 3,
      state: "active",
      source: "assistant-reported",
      category: "correctness",
      references: {
        good_example: "src/auth/refresh.ts:88",
        bad_example:  "src/auth/session.ts:142",
        evidence:     ["s-2895"]
      },
      learned: "2026-03-02",
      reinforced: 1,
      violated: 0,
      lastRelevant: "4 days ago"
    },
    {
      id: "m-leading-comma-sql",
      what: "SQL uses leading-comma style; DDL column types align at col 27.",
      because: "House style, enforced by the SQL linter. Every PR with trailing-comma or free-form alignment bounced.",
      scope: { level: "global" },
      strength: 5,
      state: "battle-tested",
      source: "preference",
      category: "convention",
      references: {
        good_example: "migrations/2026-01-ddl-users.sql:3",
        doc:          "docs/sql-style.md#columns"
      },
      learned: "2025-09-04",
      reinforced: 8,
      violated: 1,
      lastRelevant: "3 days ago"
    },
    {
      id: "m-result-api",
      what: "API handlers return Result<Json<T>, ApiError>.",
      because: "Unified error envelope — the frontend relies on ApiError.code for retry logic.",
      scope: { level: "project", project: "lumen", modules: ["handlers/*"], taskTypes: ["feat"] },
      strength: 4,
      state: "reinforced",
      source: "correction",
      category: "convention",
      references: {
        good_example: "src/handlers/billing.rs:31",
        pattern:      "result-envelope",
        evidence:     ["s-2601", "s-2733"]
      },
      learned: "2026-02-10",
      reinforced: 3,
      violated: 2,
      lastRelevant: "1 day ago"
    },
    {
      id: "m-no-trailing-summaries",
      what: "Terse responses. No trailing summaries.",
      because: "User preference, reinforced 12 times. Every 'in conclusion' or 'to summarize' paragraph got deleted.",
      scope: { level: "global" },
      strength: 5,
      state: "battle-tested",
      source: "preference",
      category: "preference",
      references: { evidence: ["s-1820", "s-1877", "s-2001", "s-2204"] },
      learned: "2025-07-11",
      reinforced: 12,
      violated: 0,
      lastRelevant: "today"
    },
    {
      id: "m-dont-docstring-unchanged",
      what: "Don't add docstrings to code you didn't change.",
      because: "Drift. Doctrine says 'docstrings = the diff' — scope creep in reviews.",
      scope: { level: "global" },
      strength: 4,
      state: "reinforced",
      source: "correction",
      category: "preference",
      references: { evidence: ["s-2104", "s-2312"] },
      learned: "2025-10-18",
      reinforced: 5,
      violated: 0,
      lastRelevant: "5 days ago"
    },
    {
      id: "m-verify-render",
      what: "Verify rendered output — not just the API response.",
      because: "Three times in a row, 200 OK + broken UI. The response was fine; the rendered state was stale.",
      scope: { level: "global" },
      strength: 3,
      state: "active",
      source: "correction",
      category: "correctness",
      references: { evidence: ["s-2544", "s-2570"] },
      learned: "2026-02-28",
      reinforced: 2,
      violated: 1,
      lastRelevant: "a week ago"
    },
    {
      id: "m-crdt-commutative",
      what: "All CRDT ops in sync/crdt.ts must be commutative.",
      because: "Non-commutative ordering made two clients diverge on out-of-order delivery. Took 5 hours to reproduce.",
      scope: { level: "module", project: "koto", modules: ["sync/crdt.ts"], taskTypes: ["feat", "fix"] },
      strength: 4,
      state: "reinforced",
      source: "assistant-reported",
      category: "correctness",
      references: {
        good_example: "src/sync/ops/insert.ts:22",
        bad_example:  "src/sync/ops/move.ts:58",
        evidence:     ["s-2720"]
      },
      learned: "2026-03-14",
      reinforced: 1,
      violated: 0,
      lastRelevant: "12 days ago"
    },
    {
      id: "m-rust-axum-subrouters",
      what: "Mount sub-routers per domain, not a single flat router.",
      because: "Flat routers made the match table O(routes) per request; sub-routers scope middleware correctly (auth, rate-limit).",
      scope: { level: "stack", stack: ["rust", "axum"] },
      strength: 3,
      state: "active",
      source: "learned",
      category: "pattern",
      references: {
        good_example: "src/router.rs:40",
        pattern:      "domain-subrouter",
        doc:          "docs/routing.md"
      },
      learned: "2026-01-30",
      reinforced: 2,
      violated: 0,
      lastRelevant: "1 week ago"
    },
    {
      id: "m-svelte5-state",
      what: "Use $state not `let` for reactivity in Svelte 5.",
      because: "Legacy `let` reactivity silently stops updating across component boundaries in Svelte 5.",
      scope: { level: "stack", stack: ["svelte@5"] },
      strength: 2,
      state: "active",
      source: "correction",
      category: "convention",
      references: { evidence: ["s-2780"] },
      learned: "2026-04-01",
      reinforced: 0,
      violated: 3,
      lastRelevant: "1 day ago"
    },
    {
      id: "m-old-caching-header",
      what: "Set Cache-Control: no-store on auth endpoints.",
      because: "(archived — superseded by stricter CSP policy in 2026-03)",
      scope: { level: "project", project: "lumen", modules: ["auth/*"], taskTypes: ["feat"] },
      strength: 0,
      state: "archived",
      source: "correction",
      category: "correctness",
      references: { related: ["m-auth-csp"] },
      learned: "2025-11-02",
      reinforced: 1,
      violated: 0,
      lastRelevant: "6 weeks ago"
    }
  ];

  // ── Patterns detected across the code ────────────────────
  const patterns = [
    {
      id: "p-adapter",
      name: "adapter",
      kind: "emerging",
      confidence: 0.92,
      occurrences: 7,
      projects: ["lumen"],
      ftrDelta: 0.18,
      desc: "Adapter wraps a third-party SDK behind a trait. Seen in 7 places; auth sessions that used it had +18% FTR.",
      sample: "src/middleware/auth_adapter.rs:14",
      memoryId: "m-auth-adapter",
      status: "promote-candidate"
    },
    {
      id: "p-retry-backoff",
      name: "retry-with-backoff",
      kind: "emerging",
      confidence: 0.78,
      occurrences: 4,
      projects: ["lumen", "tabi"],
      ftrDelta: 0.11,
      desc: "Exponential backoff with jitter in http clients. Seen in 4 places, copy-pasted — ripe to extract.",
      sample: "src/http/retry.rs:9",
      status: "unclaimed"
    },
    {
      id: "p-result-envelope",
      name: "result-envelope",
      kind: "adopted",
      confidence: 0.96,
      occurrences: 23,
      projects: ["lumen"],
      ftrDelta: 0.22,
      desc: "Result<Json<T>, ApiError> envelope across all handlers.",
      sample: "src/handlers/billing.rs:31",
      memoryId: "m-result-api",
      status: "adopted"
    },
    {
      id: "p-duplicate-error-handling",
      name: "duplicate-error-handling",
      kind: "anti",
      confidence: 0.88,
      occurrences: 12,
      projects: ["lumen"],
      ftrDelta: -0.14,
      desc: "Error-to-ApiError conversion copy-pasted across 12 handlers. Diverges quickly — already two places map the same error differently.",
      sample: "src/handlers/payments.rs:78",
      status: "antipattern"
    },
    {
      id: "p-inline-sql",
      name: "inline-sql-in-handler",
      kind: "anti",
      confidence: 0.72,
      occurrences: 5,
      projects: ["tabi"],
      ftrDelta: -0.09,
      desc: "Raw SQL strings inside HTTP handlers. Should live in a repository layer — already caused two SQL injection scares.",
      sample: "src/handlers/users.rs:140",
      status: "antipattern"
    }
  ];

  // ── Recurring corrections ────────────────────────────────
  const corrections = [
    {
      id: "c-svelte-state",
      text: "Rewrite `let x = …` → `let x = $state(…)` in Svelte components.",
      count: 6,
      lastSeen: "today",
      projects: ["koto"],
      memoryId: "m-svelte5-state",
      suggestion: "Reinforce the memory to strength 4 + add to the svelte@5 stack rules."
    },
    {
      id: "c-result-api",
      text: "Change `return res.json(...)` to `Ok(Json(...))`.",
      count: 4,
      lastSeen: "yesterday",
      projects: ["lumen"],
      memoryId: "m-result-api",
      suggestion: "Memory exists. Promote to a hard rule in the lumen skill."
    },
    {
      id: "c-trailing-summary",
      text: "Delete trailing 'in summary' paragraphs from assistant responses.",
      count: 3,
      lastSeen: "2 days ago",
      projects: ["all"],
      memoryId: "m-no-trailing-summaries",
      suggestion: "Memory already at strength 5. Consider a global response-style skill."
    }
  ];

  // ── Recommendations (the inbox) ──────────────────────────
  // Each rec: kind, title, reasoning, action, target (what it touches),
  // based on which memories/patterns/corrections.
  const recommendations = [
    {
      id: "r-promote-adapter",
      kind: "promote-pattern",
      title: "Promote the adapter pattern to a project rule.",
      reasoning: "Seen in 7 places across lumen-cloud · FTR +18% where applied · memory has been reinforced 2× · no violations in 6 weeks.",
      action: "Promote to rule",
      targetKind: "skill",
      targetName: "lumen-conventions.skill.md",
      basedOn: { patterns: ["p-adapter"], memories: ["m-auth-adapter"] },
      impact: "high"
    },
    {
      id: "r-agent-auth-tests",
      kind: "create-agent",
      title: "Create an `auth-tests` agent.",
      reasoning: "6 sessions this month in auth/* with low FTR (0.64) · 4 corrections mapped to the same 3 memories · you always run the same 5 steps (snapshot → mock inbox → run suite → diff DDL → commit).",
      action: "Draft agent",
      targetKind: "agent",
      targetName: "auth-tests.agent.md",
      basedOn: { memories: ["m-inflight-mutex", "m-no-mock-db", "m-auth-adapter"] },
      impact: "high"
    },
    {
      id: "r-skill-backoff",
      kind: "write-skill",
      title: "Write a `retry-with-backoff` skill.",
      reasoning: "Pattern detected in 4 places across 2 projects · copy-pasted, no shared abstraction · +11% FTR on sessions that used it.",
      action: "Scaffold skill",
      targetKind: "skill",
      targetName: "retry-backoff.skill.md",
      basedOn: { patterns: ["p-retry-backoff"] },
      impact: "medium"
    },
    {
      id: "r-archive-caching",
      kind: "archive-memory",
      title: "Archive the Cache-Control memory.",
      reasoning: "Superseded by the CSP-v3 policy (2026-03). Strength decayed to 0 · no relevance in 6 weeks · related memory already supersedes.",
      action: "Archive",
      targetKind: "memory",
      targetName: "m-old-caching-header",
      basedOn: { memories: ["m-old-caching-header"] },
      impact: "low"
    },
    {
      id: "r-transfer-adapter",
      kind: "cross-project",
      title: "Consider the adapter pattern in tabi-sdk.",
      reasoning: "You use it in lumen (FTR +18%). tabi-sdk has 5 inline auth call-sites with the same shape — `p-duplicate-error-handling` is flagging them.",
      action: "Clone to project",
      targetKind: "project",
      targetName: "tabi-sdk",
      basedOn: { memories: ["m-auth-adapter"], patterns: ["p-duplicate-error-handling"] },
      impact: "medium"
    },
    {
      id: "r-supersede-svelte",
      kind: "enrich-memory",
      title: "Reinforce `$state` memory — strength 2 → 4.",
      reasoning: "Violated 3× this week · 6 corrections in total · at the current strength, sensei may not surface it in context assembly.",
      action: "Reinforce",
      targetKind: "memory",
      targetName: "m-svelte5-state",
      basedOn: { memories: ["m-svelte5-state"], corrections: ["c-svelte-state"] },
      impact: "high"
    }
  ];

  // ── Lifecycle events (14-day feed) ──────────────────────
  const lifecycle = [
    { id: "e-1",  when: "today",       kind: "reinforced", memoryId: "m-no-trailing-summaries", note: "Corrected once in s-2905; strength stays at 5." },
    { id: "e-2",  when: "today",       kind: "violated",   memoryId: "m-svelte5-state",         note: "3rd violation in 7 days — sensei is flagging this." },
    { id: "e-3",  when: "yesterday",   kind: "reinforced", memoryId: "m-result-api",            note: "User corrected a handler in s-2904." },
    { id: "e-4",  when: "2 days ago",  kind: "learned",    memoryId: "m-auth-adapter",          note: "Reinforced from s-2901 — now strength 4." },
    { id: "e-5",  when: "4 days ago",  kind: "challenged", memoryId: "m-rust-axum-subrouters",  note: "Assistant proposed an alternative; user didn't resolve." },
    { id: "e-6",  when: "1 week ago",  kind: "archived",   memoryId: "m-old-caching-header",    note: "Auto-archive candidate (strength 0; no reference ≥6 weeks)." },
    { id: "e-7",  when: "2 weeks ago", kind: "superseded", memoryId: "m-old-caching-header",    note: "Replaced by CSP-v3 memory." }
  ];

  // ── Top-line counters ───────────────────────────────────
  const active = memories.filter(m => m.state !== "archived");
  const counts = {
    memories:       active.length,
    patterns:       patterns.length,
    corrections:    corrections.length,
    recs:           recommendations.length,
    archived:       memories.length - active.length,
    ftrFromMemory:  0.08   // +8% FTR on sessions where memory was surfaced
  };

  return { projects: PROJ, memories, patterns, corrections, recommendations, lifecycle, counts };
})();
