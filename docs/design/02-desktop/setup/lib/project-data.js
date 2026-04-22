// Data for Project page, Libraries area, and Navigation screens.
// Reads from + extends SENSEI_DATA when useful.

window.PROJECT_DATA = {
  // ── Focus project used in the project-page variations ────────────────
  active: "lumen-cloud",

  projects: {
    "lumen-studio": {
      id: "lumen-studio", kanji: "工", name: "Lumen Studio",
      client: "internal", goal: "A desktop design tool that feels like paper.",
      icon: { kind: "kanji", value: "工", bg: "var(--shu-soft)", fg: "var(--shu)" },
      stack: {
        languages: ["TypeScript", "Rust"],
        frameworks: ["Svelte", "Tauri"],
        runtimes: ["Node 20", "wgpu"],
        services: []
      },
      repos: [
        { id: "lumen-app",    path: "~/work/lumen/lumen-app",    stars: 0, size: "42k LOC", lang: "Svelte · TS" },
        { id: "lumen-canvas", path: "~/work/lumen/lumen-canvas", stars: 0, size: "28k LOC", lang: "Rust · WGSL" },
        { id: "lumen-shell",  path: "~/work/lumen/lumen-shell",  stars: 0, size: "6k LOC",  lang: "Rust · Tauri" }
      ],
      ftr: 0.82, ftrPrev: 0.74, sessions7d: 41,
      preferredAcp: "claude-code"
    },
    "lumen-cloud": {
      id: "lumen-cloud", kanji: "雲", name: "Lumen Cloud",
      client: "internal",
      goal: "Keep everyone's work in sync without a central server feeling central.",
      icon: { kind: "kanji", value: "雲", bg: "var(--jade-soft)", fg: "var(--jade)" },
      stack: {
        languages: ["Rust", "TypeScript"],
        frameworks: ["axum", "sqlx"],
        runtimes: ["tokio 1.36"],
        services: ["PostgreSQL 15", "Redis 7", "Stripe", "GitHub"]
      },
      repos: [
        { id: "lumen-api",    path: "~/work/lumen/lumen-api",    size: "18k LOC", lang: "Rust · Axum" },
        { id: "lumen-sync",   path: "~/work/lumen/lumen-sync",   size: "11k LOC", lang: "Rust · CRDT"  },
        { id: "lumen-auth",   path: "~/work/lumen/lumen-auth",   size: "7k LOC",  lang: "Rust · OAuth" }
      ],
      ftr: 0.64, ftrPrev: 0.71, sessions7d: 28,
      preferredAcp: "claude-code",
      ftr14: [0.71, 0.73, 0.70, 0.68, 0.65, 0.69, 0.67, 0.64, 0.62, 0.66, 0.63, 0.61, 0.65, 0.64]
    },
    "brand-kit": {
      id: "brand-kit", kanji: "紋", name: "Brand Kit",
      client: "internal",
      goal: "A single source of truth for color, type, and motion.",
      icon: { kind: "kanji", value: "紋", bg: "var(--amber-soft)", fg: "var(--amber)" },
      stack: {
        languages: ["TypeScript"],
        frameworks: ["Vite", "Style Dictionary"],
        runtimes: ["Node 20"],
        services: []
      },
      repos: [
        { id: "brand-tokens", path: "~/work/lumen/brand-tokens", size: "3k LOC",  lang: "TypeScript" },
        { id: "brand-docs",   path: "~/work/lumen/brand-docs",   size: "Markdown", lang: "MDX" }
      ],
      ftr: 0.91, ftrPrev: 0.88, sessions7d: 12,
      preferredAcp: "cursor"
    }
  },

  // Settings for the focus project
  settings: {
    links: [
      { id: "l1", kind: "docs",      label: "Lumen Cloud design doc",     url: "notion.so/lumen/cloud-dd" },
      { id: "l2", kind: "dashboard", label: "Grafana · auth-service",     url: "grafana.lumen/auth" },
      { id: "l3", kind: "issues",    label: "Linear · Cloud team",        url: "linear.app/lumen/cloud" },
      { id: "l4", kind: "runbook",   label: "Incident runbook",           url: "notion.so/lumen/cloud-runbook" }
    ],
    guidelines: [
      { id: "g1", rule: "All new handlers go through the middleware registry, not the router directly." },
      { id: "g2", rule: "No secrets in env.example — use placeholder sigils (⌘, §)." },
      { id: "g3", rule: "CRDT ops must be commutative; add a conformance test for every new op." }
    ],
    backlog: [
      { id: "b1", task: "Extract refresh-token code into its own crate", added: "4d ago" },
      { id: "b2", task: "Replace JSON logs with OTLP spans",             added: "1w ago" },
      { id: "b3", task: "Decide: push-notification strategy",            added: "2w ago" }
    ],
    skills: [
      { id: "test-gen",        name: "Test generation",       on: true  },
      { id: "pattern-extract", name: "Pattern extraction",    on: true  },
      { id: "doc-drift",       name: "Doc drift detection",   on: false },
      { id: "refactor-warn",   name: "Refactor target warn",  on: true  },
      { id: "session-coach",   name: "Session coaching",      on: true  },
      { id: "lib-docs",        name: "Library docs (MCP)",    on: true  }
    ],
    excluded: ["**/target/**", "**/node_modules/**", "**/*.lock", "/fixtures/**"],
    privacy: {
      logPrompts: true,
      logFileContents: "summaries-only",
      redactSecrets: true,
      shareWithCloud: false
    }
  },

  // ── Code graph data (shared across all three lens views) ─────────────
  // Nodes represent files, tagged with per-overlay attributes.
  graph: {
    nodes: [
      // lumen-api
      { id: "api/router.ts",      repo: "lumen-api",  fan: 42, rework: 7, stale: 2,  pattern: "router",    group: 0, hot: true,  x: 0.70, y: 0.38, size: 2.2 },
      { id: "api/middleware.ts",  repo: "lumen-api",  fan: 18, rework: 2, stale: 4,  pattern: "middleware",group: 0, hot: false, x: 0.58, y: 0.28, size: 1.3 },
      { id: "api/handlers/auth.ts",repo: "lumen-api", fan: 11, rework: 6, stale: 1,  pattern: "handler",   group: 0, hot: true,  x: 0.66, y: 0.52, size: 1.4,  dup: "auth-handler" },
      { id: "api/handlers/sync.ts",repo: "lumen-api", fan: 9,  rework: 1, stale: 0,  pattern: "handler",   group: 0, hot: false, x: 0.78, y: 0.54, size: 1.2 },
      // lumen-auth
      { id: "auth/session.ts",    repo: "lumen-auth", fan: 28, rework: 5, stale: 1,  pattern: "session",   group: 1, hot: true,  x: 0.30, y: 0.62, size: 1.9 },
      { id: "auth/refresh.ts",    repo: "lumen-auth", fan: 8,  rework: 8, stale: 0,  pattern: "token",     group: 1, hot: true,  x: 0.22, y: 0.54, size: 1.4,  dup: "auth-handler" },
      { id: "auth/device-flow.ts",repo: "lumen-auth", fan: 4,  rework: 4, stale: 0,  pattern: "flow",      group: 1, hot: false, x: 0.15, y: 0.65, size: 1.0 },
      { id: "auth/oauth.ts",      repo: "lumen-auth", fan: 6,  rework: 0, stale: 12, pattern: "provider",  group: 1, hot: false, x: 0.20, y: 0.78, size: 1.1 },
      // lumen-sync
      { id: "sync/crdt.ts",       repo: "lumen-sync", fan: 14, rework: 1, stale: 0,  pattern: "crdt",      group: 2, hot: false, x: 0.46, y: 0.22, size: 1.5 },
      { id: "sync/reconcile.ts",  repo: "lumen-sync", fan: 7,  rework: 2, stale: 1,  pattern: "crdt",      group: 2, hot: false, x: 0.52, y: 0.15, size: 1.1 },
      { id: "sync/transport.ts",  repo: "lumen-sync", fan: 5,  rework: 0, stale: 8,  pattern: "transport", group: 2, hot: false, x: 0.40, y: 0.12, size: 1.0 },
      { id: "sync/clock.ts",      repo: "lumen-sync", fan: 3,  rework: 0, stale: 21, pattern: "clock",     group: 2, hot: false, x: 0.36, y: 0.18, size: 0.9 }
    ],
    edges: [
      ["api/router.ts", "api/middleware.ts"],
      ["api/router.ts", "api/handlers/auth.ts"],
      ["api/router.ts", "api/handlers/sync.ts"],
      ["api/handlers/auth.ts", "auth/session.ts"],
      ["api/handlers/auth.ts", "auth/refresh.ts"],
      ["auth/session.ts", "auth/refresh.ts"],
      ["auth/session.ts", "auth/oauth.ts"],
      ["auth/refresh.ts", "auth/device-flow.ts"],
      ["auth/oauth.ts", "auth/device-flow.ts"],
      ["api/handlers/sync.ts", "sync/crdt.ts"],
      ["sync/crdt.ts", "sync/reconcile.ts"],
      ["sync/crdt.ts", "sync/transport.ts"],
      ["sync/transport.ts", "sync/clock.ts"],
      ["sync/reconcile.ts", "sync/clock.ts"]
    ],
    // Duplicate-cluster metadata
    duplicates: [
      {
        id: "auth-handler",
        title: "Auth-handler shape repeats",
        confidence: 0.86,
        files: ["api/handlers/auth.ts", "auth/refresh.ts"],
        sketch: "both wrap handler body in a try/catch + map errors to ApiError"
      }
    ]
  },

  // Patterns — two sides: patterns in use (follow) and anti-patterns (avoid).
  // Some anti-patterns include `suggest` pointing to a constructive pattern
  // that would fix the issue — sensei surfaces those as cross-links.
  patterns: {
    followed: [
      { id: "p1", kanji: "紋", name: "Adapter",
        family: "GoF · structural",
        places: 7, recent: "4 files in last 7d", confidence: 0.92, status: "rule",
        summary: "Wraps the legacy auth SDK so the rest of the app sees one interface.",
        example: "struct LegacySdkAdapter(LegacySdk);\nimpl AuthProvider for LegacySdkAdapter { … }",
        file: "api/middleware.ts",
        enforcement: "Promoted to project rule · new auth integrations must adapt, not inline."
      },
      { id: "p2", kanji: "紋", name: "Observer",
        family: "GoF · behavioral",
        places: 14, recent: "used across all CRDT ops", confidence: 0.97, status: "rule",
        summary: "CRDT ops notify subscribers on apply. Property test enforces commutativity.",
        example: "prop_test!(fn commutes(a, b) { apply(a,b) == apply(b,a) });\nops.subscribe(|delta| { /* fan-out */ });",
        file: "sync/crdt.ts",
        enforcement: "Rule: every op must emit a Delta and pass the commutativity prop-test."
      },
      { id: "p3", kanji: "紋", name: "Factory",
        family: "GoF · creational",
        places: 5, recent: "3 handlers + 2 tools", confidence: 0.88, status: "rule",
        summary: "ToolFactory constructs canvas tools from a registry key. Avoids switch/case sprawl.",
        example: "ToolFactory::create('freehand') // → FreehandTool { smooth: BezierSmoother }",
        file: "canvas/tools/factory.ts"
      },
      { id: "p4", kanji: "紋", name: "Repository",
        family: "patterns.dev · data",
        places: 4, recent: "lumen-api handlers", confidence: 0.84, status: "rule",
        summary: "All DB access goes through `*Repository` traits. Tests mock the trait.",
        example: "trait SessionRepository { async fn find(id: Id) -> Option<Session>; … }",
        file: "api/repos/mod.rs"
      },
      { id: "p5", kanji: "繰", name: "Retry with exponential backoff",
        family: "resilience",
        places: 3, recent: "emerging — s-2891 · s-2886", confidence: 0.68, status: "suggested",
        summary: "Starting to appear around refresh-token flows. Not yet a project rule.",
        example: "if err.is_skew() && tries < 2 { sleep(2.pow(tries) * base); retry(); }",
        file: "auth/refresh.ts"
      },
      { id: "p6", kanji: "紋", name: "Circuit Breaker",
        family: "resilience · microservices",
        places: 0, recent: "absent", confidence: 0.0, status: "gap",
        summary: "lumen-sync talks to 3 external services with no breaker. Transient outages cascade.",
        example: "// Missing: CircuitBreaker::wrap(upstream_client)",
        file: "sync/upstream.ts",
        enforcement: "Recommended — adopt for every outbound HTTP client in lumen-sync."
      }
    ],
    antiPatterns: [
      { id: "a1", kanji: "双", name: "Duplicated authentication guard",
        type: "duplication",
        severity: "high",
        occurrences: 4,
        places: ["api/handlers/auth.ts:42",
                 "api/handlers/sync.ts:18",
                 "api/handlers/users.ts:31",
                 "api/handlers/billing.ts:55"],
        summary: "Same `require_auth()` body re-implemented in 4 handlers. Diverges subtly — sync.ts forgets audit logging.",
        example: "// handlers/auth.ts\nif (!req.user) return 401;\naudit('auth.read', req);\n\n// handlers/sync.ts\nif (!req.user) return 401;\n// ← forgets audit",
        suggest: { patternId: "p1", name: "Adapter / Middleware",
                    reason: "Extract into `withAuth(handler)` middleware — one check, one audit, applied once." }
      },
      { id: "a2", kanji: "巨", name: "God node · lumen-api/src/router.ts",
        type: "god-node",
        severity: "high",
        occurrences: 1,
        places: ["api/src/router.ts (42 fan-in · 18 fan-out · 412 LoC)"],
        summary: "Router owns routing, auth, serialization, error mapping, and logging. 3 rework sessions touched it in 7d.",
        example: "// 412 lines, one file:\nrouter.post('/auth/login', login);\nrouter.post('/sync/apply', apply);\n// …43 more routes, inline auth checks, inline error mapping",
        suggest: { patternId: "p4", name: "Repository + Adapter",
                    reason: "Split into route modules per resource; push concerns into middleware (auth, logging, error-map) and repositories (data)." }
      },
      { id: "a3", kanji: "双", name: "Duplicated retry logic",
        type: "duplication",
        severity: "medium",
        occurrences: 3,
        places: ["auth/refresh.ts:88",
                 "sync/upstream.ts:120",
                 "api/handlers/stripe.ts:66"],
        summary: "Three places hand-roll the same retry-with-delay. Two use different base timeouts, one has no jitter.",
        example: "// refresh.ts: 3 attempts, 500ms base, no jitter\n// upstream.ts: 5 attempts, 200ms base, jitter\n// stripe.ts:   3 attempts, 1s base, no jitter",
        suggest: { patternId: "p5", name: "Retry with exponential backoff",
                    reason: "You already have a suggested Retry pattern emerging. Promote it and replace the 3 inlined versions." }
      },
      { id: "a4", kanji: "塊", name: "Monolithic module · session.ts",
        type: "monolith",
        severity: "medium",
        occurrences: 1,
        places: ["auth/session.ts (312 LoC · 11 responsibilities)"],
        summary: "Session lifecycle, rotation, invalidation, cache, audit log, and rate-limiter all in one file. Every auth bug lives here.",
        example: "// session.ts responsibilities:\n// create · validate · rotate · invalidate\n// · cache · log · rate-limit · notify · …",
        suggest: { patternId: "p2", name: "Observer + Repository split",
                    reason: "Split into SessionRepository (persistence) + SessionEvents (pub/sub). Cross-cutting concerns subscribe." }
      },
      { id: "a5", kanji: "死", name: "Dead code · old middleware",
        type: "dead-code",
        severity: "low",
        occurrences: 2,
        places: ["api/middleware/legacy-auth.ts (0 refs)",
                 "api/middleware/v1-logger.ts (0 refs)"],
        summary: "Two middleware files with zero import references. Last touched 4 months ago.",
        example: "// legacy-auth.ts — exported but nothing imports it.",
        suggest: null
      },
      { id: "a6", kanji: "繰", name: "Copy-paste error handling",
        type: "duplication",
        severity: "medium",
        occurrences: 12,
        places: ["12 sites across lumen-api"],
        summary: "`try/catch → log → return 500` pasted in 12 handlers. You already have a `withErrorMap` adapter — it's just not used consistently.",
        example: "// handler 1-12:\ntry { /* … */ } catch (e) {\n  logger.error(e); return res.status(500).json({err});\n}",
        suggest: { patternId: "p1", name: "Adapter (already defined)",
                    reason: "`withErrorMap` exists in middleware.ts. Wrap remaining handlers." }
      }
    ]
  },

  // Recommended actions — the interesting part
  recommendations: [
    {
      id: "r1",
      urgency: "high",
      kanji: "急",
      title: "Write an auth integration-test persona.",
      why: "Three sessions corrected in lumen-auth in 7 days. Pattern is clear: every correction touched refresh or device flow. No persona for this module yet.",
      impact: "Projected FTR +14% in Lumen Cloud",
      evidence: ["s-2891", "s-2889", "s-2886"],
      promptTitle: "Draft an auth-test persona",
      defaultAcp: "claude-code",
      cwd: "~/work/lumen/lumen-auth",
      prompt: `You are drafting a new Sensei *persona* for Lumen Cloud's lumen-auth module.

Context:
- 3 recent sessions (s-2891, s-2889, s-2886) corrected issues in refresh.ts, device-flow.ts.
- Common failure mode: the model forgets clock-skew tolerance and doesn't pattern-match the existing retry strategy.
- No integration-test persona exists for this module yet.

Task:
1. Read src/auth/refresh.ts and src/auth/device-flow.ts.
2. Summarize the house-style retry strategy in ≤7 bullets.
3. Draft a persona YAML under .sensei/personas/auth-tests.yaml with:
   - triggers (cwd matches)
   - pre-session rules (clock-skew, skewTolerance, inFlightMutex)
   - a test stub that exercises commutativity of concurrent refresh + rotation.
4. Run the stub. If it fails, stop and report; do not iterate.

Constraints:
- Do not modify production code.
- No new deps.`
    },
    {
      id: "r2",
      urgency: "medium",
      kanji: "繰",
      title: "The 'auth-handler' shape is duplicated.",
      why: "api/handlers/auth.ts and auth/refresh.ts share 86% structural similarity. Both wrap handler body in a try/catch and map errors to ApiError. Extractable into api/middleware.ts.",
      impact: "~140 LOC removed · single place to fix",
      evidence: ["dup-cluster · auth-handler"],
      promptTitle: "Extract shared error-wrapping helper",
      defaultAcp: "claude-code",
      cwd: "~/work/lumen/lumen-api",
      prompt: `Goal: extract duplicated auth-handler wrapping into a single helper.

Files involved:
- api/handlers/auth.ts
- auth/refresh.ts (consumes the wrapped result)

Tasks:
1. Inspect both files. Identify the shared shape — likely a \`with_auth_err<T>()\` HOF.
2. Introduce api/middleware::auth_err(handler) that wraps and maps.
3. Replace the inline wrappers in both files.
4. Keep behavior byte-identical; add one round-trip test.

Do NOT change error messages or status codes.`
    },
    {
      id: "r3",
      urgency: "low",
      kanji: "旧",
      title: "sync/clock.ts hasn't been touched in 21 days.",
      why: "Clock module is stale relative to recent clock-skew work in auth. Worth checking whether assumptions still hold.",
      impact: "~20 min audit",
      evidence: ["auth/refresh.ts touched 6×", "sync/clock.ts untouched"],
      promptTitle: "Cross-check clock.ts assumptions",
      defaultAcp: "claude-code",
      cwd: "~/work/lumen/lumen-sync",
      prompt: `Audit sync/clock.ts for drift against recent auth work.

Recent commits in lumen-auth touched clock-skew tolerance (refresh.ts).
Does sync/clock.ts make assumptions that are now inconsistent?

Deliver:
- A one-page note in docs/clock-drift.md
- Do NOT change code.`
    }
  ],

  // Per-file metadata for the list view
  files: [
    { path: "src/auth/refresh.ts",         repo: "lumen-auth", rework: 8, stale: 0,  touched: "1h ago", tags: ["hot"] },
    { path: "src/auth/session.ts",         repo: "lumen-auth", rework: 5, stale: 1,  touched: "yesterday", tags: ["god-node"] },
    { path: "src/router.ts",               repo: "lumen-api",  rework: 7, stale: 2,  touched: "yesterday", tags: ["god-node","hot"] },
    { path: "src/handlers/auth.ts",        repo: "lumen-api",  rework: 6, stale: 1,  touched: "2d ago",    tags: ["duplicate","hot"] },
    { path: "src/handlers/sync.ts",        repo: "lumen-api",  rework: 1, stale: 0,  touched: "3d ago",    tags: [] },
    { path: "src/crdt.ts",                 repo: "lumen-sync", rework: 1, stale: 0,  touched: "3d ago",    tags: ["pattern-source"] },
    { path: "src/clock.ts",                repo: "lumen-sync", rework: 0, stale: 21, touched: "21d ago",   tags: ["stale","drift-risk"] }
  ],

  // Sessions scoped to this project
  recentSessions: [
    { id: "s-2891", project: "lumen-auth",  title: "Fix refresh token rotation", time: "10:42", duration: "38m", ftr: false, corrections: 3 },
    { id: "s-2889", project: "lumen-auth",  title: "OAuth device flow",          time: "Yesterday", duration: "1h 12m", ftr: false, corrections: 4 },
    { id: "s-2886", project: "lumen-auth",  title: "Session invalidation on password change", time: "2d ago", duration: "44m", ftr: false, corrections: 2 },
    { id: "s-2885", project: "lumen-sync",  title: "CRDT merge on offline reconnect",         time: "2d ago", duration: "1h 30m", ftr: true, corrections: 0 }
  ]
};

// ── Libraries ──────────────────────────────────────────────────────────
window.LIBRARIES_DATA = {
  groups: [
    {
      id: "detected", kanji: "見", label: "Detected",
      sub: "discovered from package.json · Cargo.toml · imports",
      items: [
        { id: "axum",    name: "axum",          version: "0.7.5", lang: "Rust",   docs: "indexed",  usage: 47, repos: ["lumen-api"],
          source: "Cargo.toml · lumen-api", lastIndexed: "2d ago", icon: "A" },
        { id: "tokio",   name: "tokio",         version: "1.39",  lang: "Rust",   docs: "indexed",  usage: 112, repos: ["lumen-api","lumen-sync","lumen-auth"],
          source: "Cargo.toml", lastIndexed: "5d ago", icon: "T" },
        { id: "sqlx",    name: "sqlx",          version: "0.8.0", lang: "Rust",   docs: "indexed",  usage: 23, repos: ["lumen-api"],
          source: "Cargo.toml", lastIndexed: "3d ago", icon: "S" },
        { id: "svelte",  name: "Svelte 5",      version: "5.0.0", lang: "TS",     docs: "indexed",  usage: 128, repos: ["lumen-app"],
          source: "package.json", lastIndexed: "2d ago", icon: "S" },
        { id: "tauri",   name: "Tauri",         version: "2.1.0", lang: "Rust·TS",docs: "indexed",  usage: 14, repos: ["lumen-shell"],
          source: "Cargo.toml · package.json", lastIndexed: "1w ago", icon: "T" },
        { id: "automerge",name: "Automerge",    version: "2.2",   lang: "TS",     docs: "partial",  usage: 19, repos: ["lumen-sync"],
          source: "Cargo.toml · lumen-sync", lastIndexed: "2w ago", icon: "A" },
        { id: "wgsl",    name: "wgpu",          version: "0.20",  lang: "Rust",   docs: "none",     usage: 7,  repos: ["lumen-canvas"],
          source: "Cargo.toml", lastIndexed: null, icon: "W" }
      ]
    },
    {
      id: "imported", kanji: "入", label: "Imported",
      sub: "registered by you · internal SDKs, llms.txt sources",
      items: [
        { id: "lumen-icons", name: "@lumen/icons",   version: "4.2.0", lang: "TS",    docs: "indexed", usage: 31, repos: ["lumen-app","brand-docs"],
          source: "imported · private registry", lastIndexed: "1d ago", icon: "L", internal: true },
        { id: "lumen-tokens",name: "@lumen/tokens",  version: "1.9.3", lang: "TS",    docs: "indexed", usage: 64, repos: ["lumen-app","brand-tokens","brand-docs"],
          source: "imported · monorepo", lastIndexed: "6h ago", icon: "L", internal: true },
        { id: "house-style", name: "house-style llms.txt", version: "—", lang: "docs",docs: "indexed", usage: 0,  repos: ["(workspace-wide)"],
          source: "imported · from URL", lastIndexed: "3d ago", icon: "¶", internal: true, kind: "docs" }
      ]
    },
    {
      id: "services", kanji: "繋", label: "External services",
      sub: "connected via MCP — sensei queries these on your behalf",
      items: [
        { id: "postgres", name: "Postgres",    version: "16.1", lang: "MCP", docs: "schema", usage: 8,
          source: "MCP · postgres-ro", lastIndexed: "live", icon: "P", service: true },
        { id: "stripe",   name: "Stripe",      version: "2024-06-20", lang: "MCP", docs: "indexed", usage: 3,
          source: "MCP · stripe", lastIndexed: "live", icon: "S", service: true },
        { id: "linear",   name: "Linear",      version: "API v2", lang: "MCP", docs: "schema", usage: 12,
          source: "MCP · linear", lastIndexed: "live", icon: "L", service: true }
      ]
    }
  ],

  // Focus library for the detail panel
  focus: "axum",
  details: {
    axum: {
      id: "axum", name: "axum", tagline: "Ergonomic, modular Rust web framework · built with tokio, tower, hyper.",
      version: "0.7.5", lang: "Rust", docs: "indexed",
      source: "Cargo.toml · lumen-api", lastIndexed: "2d ago",
      pages: 94,
      summary: "Lumen Cloud's HTTP layer. Every request goes through axum's router → tower middleware stack.",
      usage: {
        calls: 47,
        topSymbols: [
          { symbol: "axum::Router::new",  n: 18 },
          { symbol: "axum::extract::State", n: 12 },
          { symbol: "axum::Json",         n: 9 },
          { symbol: "axum::middleware::from_fn", n: 6 },
          { symbol: "axum::routing::get", n: 2 }
        ],
        places: [
          { file: "lumen-api/src/router.ts",       line: 14,  snippet: "let app = Router::new()" },
          { file: "lumen-api/src/handlers/auth.ts",line: 8,   snippet: "pub async fn login(State(db): State<Db>) -> Result<Json<Session>, ApiError>" },
          { file: "lumen-api/src/middleware.ts",   line: 22,  snippet: "Router::new().layer(from_fn(trace))" }
        ]
      },
      rules: [
        { rule: "Wrap every handler in ApiError — never return Result<_, anyhow>.", source: "house-style" },
        { rule: "Mount sub-routers per domain, not per version.",                    source: "session s-2780" }
      ],
      sessions: ["s-2891", "s-2886"],
      // MCP example interactions — the "what sensei can do with this lib"
      mcpExamples: [
        {
          tool: "sensei.library.explain",
          intent: "Explain how axum routes a POST /login.",
          request: `{ "library": "axum", "path": "POST /login", "cwd": "lumen-api" }`,
          response: `axum dispatches /login through:
  Router::new()
    .route("/login", post(handlers::auth::login))
    .layer(from_fn(trace))

handlers::auth::login expects State<Db> + Json<LoginReq>.
Returns Result<Json<Session>, ApiError> — matches house-style rule.

(Source: 3 files · 2 sessions · axum docs v0.7.5)`
        },
        {
          tool: "sensei.library.find-usage",
          intent: "Where do we use axum::middleware::from_fn?",
          request: `{ "library": "axum", "symbol": "middleware::from_fn" }`,
          response: `6 call sites across lumen-api:
  • src/middleware.ts:22   trace_middleware
  • src/middleware.ts:38   auth_middleware
  • src/router.ts:19       (applied globally)
  • src/handlers/sync.ts:41 sync_rate_limit
  • ... +2 more

All wrap the handler result with .map_err(ApiError::from).
Pattern matches house-style rule #147.`
        },
        {
          tool: "sensei.library.suggest-rule",
          intent: "Is there a rule we should learn for axum?",
          request: `{ "library": "axum" }`,
          response: `Suggested rule (confidence 0.86):

  "Route handlers must return Result<Json<T>, ApiError>."

Evidence:
  • 18/18 handlers follow this shape.
  • 3 sessions (s-2891, s-2887, s-2780) corrected the one deviation.

Action:
  [ Promote to project rule ]
  [ Attach to library 'axum' ]`
        },
        {
          tool: "sensei.library.doc-drift",
          intent: "Have our axum usages drifted from the docs?",
          request: `{ "library": "axum", "since": "7d" }`,
          response: `No drift detected.

Docs indexed v0.7.5 · 2d ago.
You're using 12 symbols. All match signatures in the indexed docs.
Nothing deprecated since 0.7.3.`
        }
      ]
    }
  }
};

// ── Projects index (for navigation screens) ──────────────────────────
window.PROJECTS_INDEX = {
  projects: [
    { id: "lumen-studio", kanji: "工", name: "Lumen Studio", client: "internal",     status: "active",   ftr: 0.82, sessions7d: 41, repos: 3, libs: 14,  lastSession: "22m ago", warn: false },
    { id: "lumen-cloud",  kanji: "雲", name: "Lumen Cloud",  client: "internal",     status: "active",   ftr: 0.64, sessions7d: 28, repos: 3, libs: 18,  lastSession: "2h ago",  warn: true  },
    { id: "brand-kit",    kanji: "紋", name: "Brand Kit",    client: "internal",     status: "active",   ftr: 0.91, sessions7d: 12, repos: 2, libs: 9,   lastSession: "yesterday", warn: false },
    { id: "sketch-tool",  kanji: "筆", name: "Sketch tool",  client: "internal",     status: "recent",   ftr: 0.71, sessions7d: 0,  repos: 1, libs: 6,   lastSession: "3w ago",  warn: false },
    { id: "old-docs",     kanji: "巻", name: "Docs site",    client: "external",     status: "recent",   ftr: 0.88, sessions7d: 0,  repos: 1, libs: 4,   lastSession: "2mo ago", warn: false },
    { id: "prototype-x",  kanji: "試", name: "Prototype X",  client: "research",     status: "archived", ftr: 0.55, sessions7d: 0,  repos: 1, libs: 2,   lastSession: "6mo ago", warn: false }
  ],
  clients: ["internal", "external", "research"]
};

// ── MCP Tools catalog ───────────────────────────────────────────────
// Sensei exposes these tools over MCP. Assistants call them; users try
// them in the Playground. Tools declare their required inputs explicitly.
window.MCP_TOOLS = {
  categories: [
    { id: "project", kanji: "場", label: "Project tools",
      sub: "Operate on a project or repo." },
    { id: "library", kanji: "庫", label: "Library tools",
      sub: "Answer questions about a specific library." },
    { id: "pattern", kanji: "紋", label: "Pattern tools",
      sub: "Inspect and promote patterns." },
    { id: "session", kanji: "録", label: "Session tools",
      sub: "Query past sessions and outcomes." }
  ],

  tools: [
    // ─── project tools ────────────────────────────────────────
    {
      id: "project.explain-structure",
      name: "sensei.project.explain-structure",
      category: "project",
      summary: "Describe the shape of a project: repos, services, boundaries.",
      inputs: [
        { key: "project", kind: "project", required: true,
          label: "Project", help: "Which project to analyze." }
      ],
      example: {
        project: "lumen-cloud",
        response: `lumen-cloud is 3 repos:
  • lumen-api    — axum HTTP service (Rust)
  • lumen-auth   — session / refresh-token service (Rust)
  • lumen-sync   — CRDT sync service (Rust + TS client)

Shared deps: tokio, sqlx, axum.
Boundary: auth publishes SessionEvent; sync subscribes.
Entry points: api/src/main.rs, auth/src/main.rs, sync/src/main.rs.`
      }
    },
    {
      id: "project.find-hotspots",
      name: "sensei.project.find-hotspots",
      category: "project",
      summary: "Surface god-nodes, high-rework files, and monoliths.",
      inputs: [
        { key: "project", kind: "project", required: true, label: "Project" },
        { key: "since",   kind: "since",   required: false, label: "Since",
          options: ["24h", "7d", "30d", "all"], default: "7d" }
      ],
      example: {
        project: "lumen-cloud", since: "7d",
        response: `Hotspots in lumen-cloud (7d):

  ⚠ god-node   api/src/router.ts       fan-in 42 · 3 rework sessions
  ⚠ monolith   auth/src/session.ts     312 LoC · 11 responsibilities
  ⚠ rework     auth/src/refresh.ts     8 edits · 3 corrections
  · cluster    sync/src/upstream.ts    no circuit-breaker

Suggested actions:
  → split router.ts by resource (Repository pattern)
  → extract SessionEvents (Observer) from session.ts`
      }
    },
    {
      id: "project.find-duplication",
      name: "sensei.project.find-duplication",
      category: "project",
      summary: "Find copy-paste code and suggest a pattern that would fix it.",
      inputs: [
        { key: "project",   kind: "project",   required: true, label: "Project" },
        { key: "minLines",  kind: "number",    required: false, label: "Min lines",
          default: 8 }
      ],
      example: {
        project: "lumen-cloud", minLines: 8,
        response: `4 duplication clusters found:

  1. require_auth() — 4 copies in api/handlers/*.ts
     → extract Middleware adapter (withAuth)

  2. retry-with-delay — 3 copies in auth, sync, stripe handlers
     → promote emerging pattern "Retry with exponential backoff"

  3. try/catch → log → 500 — 12 copies in lumen-api
     → wrap with existing withErrorMap adapter

  4. session.invalidate_by_user — 2 copies, one incomplete
     → extract to SessionRepository`
      }
    },

    // ─── library tools ────────────────────────────────────────
    {
      id: "library.explain",
      name: "sensei.library.explain",
      category: "library",
      summary: "Explain how this library is used in your code, grounded in real call sites.",
      inputs: [
        { key: "library", kind: "library", required: true, label: "Library" },
        { key: "path",    kind: "text",    required: false, label: "Path or symbol",
          placeholder: "POST /login · Router::new · …" },
        { key: "cwd",     kind: "project", required: false, label: "In project" }
      ],
      example: {
        library: "axum", path: "POST /login", cwd: "lumen-api",
        response: `axum dispatches /login through:
  Router::new()
    .route("/login", post(handlers::auth::login))
    .layer(from_fn(trace))

handlers::auth::login expects State<Db> + Json<LoginReq>.
Returns Result<Json<Session>, ApiError> — matches house-style rule.

(Source: 3 files · 2 sessions · axum docs v0.7.5)`
      }
    },
    {
      id: "library.find-usage",
      name: "sensei.library.find-usage",
      category: "library",
      summary: "List every call site of a symbol in this library across your project.",
      inputs: [
        { key: "library", kind: "library", required: true, label: "Library" },
        { key: "symbol",  kind: "text",    required: true, label: "Symbol",
          placeholder: "middleware::from_fn" }
      ],
      example: {
        library: "axum", symbol: "middleware::from_fn",
        response: `6 call sites across lumen-api:
  • src/middleware.ts:22   trace_middleware
  • src/middleware.ts:38   auth_middleware
  • src/router.ts:19       (applied globally)
  • src/handlers/sync.ts:41 sync_rate_limit
  • ... +2 more

All wrap the handler result with .map_err(ApiError::from).
Pattern matches house-style rule #147.`
      }
    },
    {
      id: "library.suggest-rule",
      name: "sensei.library.suggest-rule",
      category: "library",
      summary: "Propose a house rule for this library based on how you actually use it.",
      inputs: [
        { key: "library", kind: "library", required: true, label: "Library" }
      ],
      example: {
        library: "axum",
        response: `Suggested rule (confidence 0.86):

  "Route handlers must return Result<Json<T>, ApiError>."

Evidence:
  • 18/18 handlers follow this shape.
  • 3 sessions (s-2891, s-2887, s-2780) corrected the one deviation.

Action:
  [ Promote to project rule ]
  [ Attach to library 'axum' ]`
      }
    },
    {
      id: "library.doc-drift",
      name: "sensei.library.doc-drift",
      category: "library",
      summary: "Check whether your usage has drifted from the library's current docs.",
      inputs: [
        { key: "library", kind: "library", required: true, label: "Library" },
        { key: "since",   kind: "since",   required: false, label: "Since",
          options: ["24h", "7d", "30d", "all"], default: "7d" }
      ],
      example: {
        library: "axum", since: "7d",
        response: `No drift detected.

Docs indexed v0.7.5 · 2d ago.
You're using 12 symbols. All match signatures in the indexed docs.
Nothing deprecated since 0.7.3.`
      }
    },

    // ─── pattern tools ────────────────────────────────────────
    {
      id: "pattern.list-followed",
      name: "sensei.pattern.list-followed",
      category: "pattern",
      summary: "Patterns you consistently use (GoF, patterns.dev, house rules).",
      inputs: [
        { key: "project", kind: "project", required: true, label: "Project" }
      ],
      example: {
        project: "lumen-cloud",
        response: `6 patterns in use:

  ✓ rule       Adapter             7 places · promoted
  ✓ rule       Observer            14 places · CRDT ops
  ✓ rule       Factory             5 places · canvas tools
  ✓ rule       Repository          4 places · api repos
  · suggested  Retry w/ backoff    3 places · emerging
  ! gap        Circuit Breaker     0 places · recommended`
      }
    },
    {
      id: "pattern.list-antipatterns",
      name: "sensei.pattern.list-antipatterns",
      category: "pattern",
      summary: "Duplication, god-nodes, monoliths, dead code — with suggested fixes.",
      inputs: [
        { key: "project",  kind: "project", required: true, label: "Project" },
        { key: "severity", kind: "enum",    required: false, label: "Min severity",
          options: ["low", "medium", "high"], default: "medium" }
      ],
      example: {
        project: "lumen-cloud", severity: "medium",
        response: `5 anti-patterns at medium+ severity:

  ⚠ high    duplication  require_auth()       4 copies  → Adapter
  ⚠ high    god-node     router.ts            412 LoC   → Repository + Adapter
  · med     duplication  retry logic          3 copies  → Retry pattern
  · med     monolith     session.ts           312 LoC   → Observer + Repository split
  · med     duplication  try/catch/500        12 sites  → withErrorMap adapter`
      }
    },
    {
      id: "pattern.promote",
      name: "sensei.pattern.promote",
      category: "pattern",
      summary: "Turn an emerging pattern into a project rule and attach evidence.",
      inputs: [
        { key: "project", kind: "project", required: true, label: "Project" },
        { key: "pattern", kind: "text",    required: true, label: "Pattern",
          placeholder: "e.g. Retry with exponential backoff" }
      ],
      example: {
        project: "lumen-cloud", pattern: "Retry with exponential backoff",
        response: `Promoted "Retry with exponential backoff" to project rule.

Created: .sensei/rules/retry-backoff.md
Evidence attached: auth/refresh.ts · sync/upstream.ts · sessions s-2891, s-2886.

Effect:
  • Future sessions that hand-roll retry will be flagged.
  • Suggestion to replace the 3 existing inlined versions queued.`
      }
    },

    // ─── session tools ────────────────────────────────────────
    {
      id: "session.summarize",
      name: "sensei.session.summarize",
      category: "session",
      summary: "Return a summary of a session: what changed, what broke, what was learned.",
      inputs: [
        { key: "session", kind: "session", required: true, label: "Session" }
      ],
      example: {
        session: "s-2891",
        response: `s-2891 · Fix refresh token rotation edge case
10:42 → 11:20 · 38m · 3 corrections · NOT first-try

Turns:
  → edited auth/refresh.ts (missed clock-skew)
  ← dev: "account for 30s clock skew per SDK 4.2"
  → added skewTolerance · tests passed
  ← dev: "handle refresh during rotation"
  → added inFlightMutex

Lesson: refresh flows need skew-tolerance + mutex by default.
Queued as rule suggestion (see sensei.pattern.promote).`
      }
    }
  ]
};
