// Wizard data + SSE-style scan events for the setup flow.
window.SENSEI_SETUP = {
  // Components state (the user wanted all three cases as sub-variants)
  componentsVariants: [
    {
      id: "fresh",
      label: "Fresh install",
      description: "Nothing installed yet — desktop will fetch and install.",
      components: [
        { id: "cli",    name: "sensei-cli",    version: null,     status: "missing", action: "install" },
        { id: "mcp",    name: "MCP bridge",    version: null,     status: "missing", action: "install" },
        { id: "daemon", name: "sensei-daemon", version: null,     status: "missing", action: "install" }
      ]
    },
    {
      id: "partial",
      label: "Partial",
      description: "CLI present. Daemon not yet started. MCP missing.",
      components: [
        { id: "cli",    name: "sensei-cli",    version: "0.9.2",  status: "installed", action: null },
        { id: "mcp",    name: "MCP bridge",    version: null,     status: "missing",   action: "install" },
        { id: "daemon", name: "sensei-daemon", version: "0.9.2",  status: "stopped",   action: "start" }
      ]
    },
    {
      id: "all-present",
      label: "All present",
      description: "Everything installed. Daemon just needs to start.",
      components: [
        { id: "cli",    name: "sensei-cli",    version: "0.9.2",  status: "installed", action: null },
        { id: "mcp",    name: "MCP bridge",    version: "0.9.2",  status: "installed", action: null },
        { id: "daemon", name: "sensei-daemon", version: "0.9.2",  status: "stopped",   action: "start" }
      ]
    }
  ],

  acps: [
    { id: "claude-code", name: "Claude Code", version: "1.8.2", found: true,  path: "/Users/aiko/.claude/code" },
    { id: "cursor",      name: "Cursor",      version: "0.42",  found: true,  path: "/Applications/Cursor.app" },
    { id: "zed",         name: "Zed",         version: "0.148", found: false, path: null },
    { id: "continue",    name: "Continue",    version: null,    found: false, path: null }
  ],

  folders: [
    { id: "f1", path: "~/code/lumen",       note: "monorepo root, 3 packages" },
    { id: "f2", path: "~/code/brand-kit",   note: "docs + tokens" },
    { id: "f3", path: "~/code/playground",  note: "sandboxes" }
  ],

  // SSE-like scan event log — used for the live scan view
  scanEvents: [
    { t: 0,    level: "info",    msg: "scan started · 3 roots · 2 workers" },
    { t: 120,  level: "discover",msg: "~/code/lumen · found git repo"   },
    { t: 180,  level: "discover",msg: "~/code/lumen/lumen-app · found git repo", parent: "lumen" },
    { t: 240,  level: "discover",msg: "~/code/lumen/lumen-canvas · found git repo", parent: "lumen" },
    { t: 310,  level: "discover",msg: "~/code/lumen/lumen-shell · found git repo", parent: "lumen" },
    { t: 380,  level: "queue",   msg: "lumen-app · 842 files queued" },
    { t: 420,  level: "queue",   msg: "lumen-canvas · 614 files queued" },
    { t: 470,  level: "queue",   msg: "lumen-shell · 291 files queued" },
    { t: 530,  level: "discover",msg: "~/code/lumen-cloud · found git repo (sibling)" },
    { t: 610,  level: "discover",msg: "~/code/lumen-cloud/lumen-api · found git repo", parent: "lumen-cloud" },
    { t: 680,  level: "discover",msg: "~/code/lumen-cloud/lumen-sync · found git repo", parent: "lumen-cloud" },
    { t: 750,  level: "discover",msg: "~/code/lumen-cloud/lumen-auth · found git repo", parent: "lumen-cloud" },
    { t: 820,  level: "process", msg: "lumen-app · 612 / 842 processed" },
    { t: 880,  level: "discover",msg: "~/code/brand-kit/brand-tokens · found git repo", parent: "brand-kit" },
    { t: 940,  level: "discover",msg: "~/code/brand-kit/brand-docs · found git repo", parent: "brand-kit" },
    { t: 1020, level: "process", msg: "lumen-canvas · 614 / 614 processed · graph extracted" },
    { t: 1080, level: "process", msg: "lumen-app · 842 / 842 processed · graph extracted" },
    { t: 1200, level: "info",    msg: "3 solutions detected · 8 repos · 3,214 files indexed" },
    { t: 1260, level: "success", msg: "scan complete · 21s" }
  ],

  // Solutions as discovered from the scan
  discoveredSolutions: [
    {
      id: "lumen-studio",
      name: "Lumen Studio",
      kanji: "工",
      path: "~/code/lumen",
      autoDetected: true,
      confidence: "high",
      projects: [
        { id: "lumen-app",    name: "lumen-app",    path: "~/code/lumen/lumen-app",    files: 842, lang: "TypeScript", suggestedRole: "frontend" },
        { id: "lumen-canvas", name: "lumen-canvas", path: "~/code/lumen/lumen-canvas", files: 614, lang: "Rust + TS",   suggestedRole: "library" },
        { id: "lumen-shell",  name: "lumen-shell",  path: "~/code/lumen/lumen-shell",  files: 291, lang: "Rust",        suggestedRole: "infra" }
      ]
    },
    {
      id: "lumen-cloud",
      name: "Lumen Cloud",
      kanji: "雲",
      path: "~/code/lumen-cloud",
      autoDetected: true,
      confidence: "high",
      projects: [
        { id: "lumen-api",  name: "lumen-api",  path: "~/code/lumen-cloud/lumen-api",  files: 412, lang: "Rust",       suggestedRole: "backend" },
        { id: "lumen-sync", name: "lumen-sync", path: "~/code/lumen-cloud/lumen-sync", files: 268, lang: "Rust",       suggestedRole: "backend" },
        { id: "lumen-auth", name: "lumen-auth", path: "~/code/lumen-cloud/lumen-auth", files: 184, lang: "Rust",       suggestedRole: "backend" }
      ]
    },
    {
      id: "brand-kit",
      name: "Brand Kit",
      kanji: "紋",
      path: "~/code/brand-kit",
      autoDetected: true,
      confidence: "medium",
      projects: [
        { id: "brand-tokens", name: "brand-tokens", path: "~/code/brand-kit/brand-tokens", files: 128, lang: "JSON + TS", suggestedRole: "library" },
        { id: "brand-docs",   name: "brand-docs",   path: "~/code/brand-kit/brand-docs",   files: 76,  lang: "Markdown",  suggestedRole: "docs" }
      ]
    }
  ],

  roles: [
    { id: "backend",  label: "Backend",    kanji: "後" },
    { id: "frontend", label: "Frontend",   kanji: "前" },
    { id: "library",  label: "Library",    kanji: "書" },
    { id: "docs",     label: "Docs",       kanji: "記" },
    { id: "infra",    label: "Infra",      kanji: "基" }
  ],

  // Detected stack — signals that drive MCP recommendations.
  // Populated during Scan by looking at manifests / Dockerfiles / env.
  detectedStack: {
    languages:  ["Rust", "TypeScript"],
    frameworks: ["axum", "sqlx", "React", "Tauri"],
    runtimes:   ["tokio 1.36", "Node 20"],
    services:   ["PostgreSQL", "Redis", "Stripe", "GitHub"]
  },

  // Libraries sensei WRAPS — these don't have an MCP of their own.
  // Typically: small / obscure / internal / undocumented.
  // Sensei indexes their code + docs and exposes its own tools over them.
  // NOT included here: anything that has a proper MCP (Postgres, Stripe, …).
  discoveredLibraries: {
    detected: [
      { id: "axum",       name: "axum",       version: "0.7.5",  lang: "Rust",
        usage: 42, source: "lumen-api/Cargo.toml",         docs: "indexed",
        why: "widely used but no MCP available · sensei wraps" },
      { id: "yjs",        name: "yjs",        version: "13.6.8", lang: "TypeScript",
        usage: 18, source: "lumen-sync/package.json",      docs: "partial",
        why: "CRDT lib · no MCP · sensei wraps" },
      { id: "sqlx",       name: "sqlx",       version: "0.7.3",  lang: "Rust",
        usage: 21, source: "lumen-api/Cargo.toml",         docs: "indexed",
        why: "used with postgres-mcp · sensei indexes the Rust-side API" },
      { id: "lumen-auth-sdk", name: "@lumen/auth-sdk", version: "internal",
        lang: "TypeScript",
        usage: 9,  source: "internal · monorepo",          docs: "schema",
        why: "internal library · no external docs · sensei wraps" },
      { id: "lumen-ops",      name: "@lumen/ops",     version: "internal",
        lang: "TypeScript",
        usage: 4,  source: "internal · monorepo",          docs: "none",
        why: "internal library · undocumented · sensei indexes code" },
      { id: "brand-tokens",   name: "@lumen/brand-tokens", version: "internal",
        lang: "TypeScript",
        usage: 14, source: "internal · monorepo",          docs: "partial",
        why: "internal library · sensei wraps tokens" }
    ]
  },

  // Global MCP registry — sensei knows about these.
  // Recommendations are computed from detectedStack at wizard time.
  // `trigger` = what in the stack surfaces this MCP as recommended.
  // `kind` = service · api · devtool · data
  mcpRegistry: {
    available: [
      { id: "postgres-mcp", name: "PostgreSQL MCP",   publisher: "supabase",
        kind: "data",       kanji: "庫",
        summary: "Query schema, introspect tables, explain plans, run safe SQL.",
        trigger: ["PostgreSQL"], tools: 14, verified: true,
        installed: true,  recommended: true },
      { id: "redis-mcp",    name: "Redis MCP",        publisher: "redis labs",
        kind: "data",       kanji: "速",
        summary: "Inspect keys, check TTLs, run diagnostics on a live instance.",
        trigger: ["Redis"],  tools: 9,  verified: true,
        installed: false, recommended: true },
      { id: "stripe-mcp",   name: "Stripe MCP",       publisher: "stripe",
        kind: "api",        kanji: "銀",
        summary: "List prices, inspect customers, dry-run webhooks from live data.",
        trigger: ["Stripe"], tools: 18, verified: true,
        installed: true,  recommended: true },
      { id: "github-mcp",   name: "GitHub MCP",       publisher: "github",
        kind: "devtool",    kanji: "貢",
        summary: "Search code, list PRs, read issues, check CI status across repos.",
        trigger: ["GitHub"], tools: 23, verified: true,
        installed: false, recommended: true },
      { id: "sentry-mcp",   name: "Sentry MCP",       publisher: "sentry.io",
        kind: "service",    kanji: "哨",
        summary: "Pull recent errors, inspect stack frames, group by release.",
        trigger: [],         tools: 11, verified: true,
        installed: false, recommended: false },
      { id: "playwright-mcp", name: "Playwright MCP", publisher: "microsoft",
        kind: "devtool",    kanji: "試",
        summary: "Run browser tests on demand, capture screenshots, inspect DOM.",
        trigger: [],         tools: 7,  verified: true,
        installed: false, recommended: false },
      { id: "figma-mcp",    name: "Figma MCP",        publisher: "figma",
        kind: "devtool",    kanji: "形",
        summary: "Read design files, pull tokens, surface component specs.",
        trigger: [],         tools: 12, verified: true,
        installed: false, recommended: false }
    ]
  },

  externalLinks: {
    autoDiscovered: [
      { id: "x1", solution: "lumen-studio", url: "https://lumen.atlassian.net/jira",           kind: "issues",     source: "lumen-app/README.md",  confidence: "high" },
      { id: "x2", solution: "lumen-studio", url: "https://lumen.atlassian.net/wiki",           kind: "wiki",       source: "package.json",         confidence: "high" },
      { id: "x3", solution: "lumen-studio", url: "https://figma.com/file/abc/Lumen-System",    kind: "design",     source: "brand-docs/README.md", confidence: "medium" },
      { id: "x4", solution: "lumen-cloud",  url: "https://dbdocs.io/lumen/cloud-schema",       kind: "db-schema",  source: "lumen-api/README.md",  confidence: "high" },
      { id: "x5", solution: "lumen-cloud",  url: "https://lumen.atlassian.net/jira/ENG",       kind: "issues",     source: "README.md",            confidence: "high" },
      { id: "x6", solution: "brand-kit",    url: "https://notion.so/lumen/brand-guidelines",   kind: "docs",       source: "README.md",            confidence: "medium" }
    ],
    availableIntegrations: [
      { id: "jira",       name: "Jira",        kanji: "問" },
      { id: "confluence", name: "Confluence",  kanji: "百" },
      { id: "notion",     name: "Notion",      kanji: "書" },
      { id: "linear",     name: "Linear",      kanji: "線" },
      { id: "github",     name: "GitHub Issues",kanji: "貢" },
      { id: "dbdocs",     name: "dbdocs",      kanji: "図" },
      { id: "figma",      name: "Figma",       kanji: "形" },
      { id: "slack",      name: "Slack",       kanji: "話" }
    ]
  },

  metadata: {
    statuses: [
      { id: "discovery",   label: "Discovery" },
      { id: "active",      label: "Active dev" },
      { id: "maintenance", label: "Maintenance" },
      { id: "archived",    label: "Archived" }
    ]
  }
};
