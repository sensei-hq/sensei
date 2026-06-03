// Wizard data + SSE-style scan events for the setup flow.
window.SENSEI_SETUP = {
  // OS-level facts the wizard derives defaults from. In a real build these
  // come from the Tauri host; here we mock a believable home path so the
  // Preferences stage can derive a display-name suggestion ("aiko" → "Aiko"),
  // matching the rest of the prototype's persona.
  system: {
    homeDir:  "/Users/aiko",
    username: "aiko",
    osName:   "macOS 14.4 · arm64",
    shell:    "zsh"
  },

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

  // Grouped by vendor "family". A family can have multiple products
  // (Claude Code + Claude Desktop both live under "Claude"). Card shows
  // the family once and lists found products as small chips.
  acps: [
    {
      id: "claude", name: "Claude", kanji: "C", found: true,
      products: [
        { id: "claude-code",    label: "Code",    version: "1.8.2", path: "~/.claude/code",    found: true },
        { id: "claude-desktop", label: "Desktop", version: "0.9.4", path: "~/Library/Application Support/Claude", found: true }
      ]
    },
    {
      id: "openai", name: "OpenAI", kanji: "O", found: true,
      products: [
        { id: "codex",   label: "Codex CLI",   version: "0.6.1", path: "~/.codex",        found: true },
        { id: "chatgpt", label: "ChatGPT app", version: "1.2025.114", path: "/Applications/ChatGPT.app", found: true }
      ]
    },
    {
      id: "cursor", name: "Cursor", kanji: "C", found: true,
      products: [
        { id: "cursor", label: "Editor", version: "0.42", path: "/Applications/Cursor.app", found: true }
      ]
    },
    {
      id: "zed", name: "Zed", kanji: "Z", found: false,
      products: [
        { id: "zed", label: "Editor", version: null, path: null, found: false }
      ]
    },
    {
      id: "jetbrains", name: "JetBrains", kanji: "J", found: false,
      products: [
        { id: "jb-junie",      label: "Junie",      version: null, path: null, found: false },
        { id: "jb-aiassistant", label: "AI Assistant", version: null, path: null, found: false }
      ]
    },
    {
      id: "continue", name: "Continue", kanji: "→", found: false,
      products: [
        { id: "continue", label: "Extension", version: null, path: null, found: false }
      ]
    }
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

  // Global MCP registry (shown as "Instruments" in the UI) — sensei knows about these.
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
  },

  // Inference — local models + external providers
  inference: {
    system: {
      chip:    "Apple M3 Max",
      cores:   "14-core CPU · 40-core GPU",
      ram:     "64 GB unified",
      ramGB:   64,
      os:      "macOS 15.1",
      disk:    "412 GB free"
    },
    localModels: [
      { id: "qwen-coder-32b",    name: "Qwen2.5-Coder 32B",    role: "inference",
        sizeGB: 19.9, tag: "32b-instruct-q5_K_M", recommended: true,
        note: "primary inference · fits comfortably on 64 GB",  pulled: false },
      { id: "deepseek-r1-14b",   name: "DeepSeek-R1 14B",      role: "consolidation",
        sizeGB: 8.4,  tag: "14b-q4_K_M",          recommended: true,
        note: "fast reasoning · memory consolidation",          pulled: false },
      { id: "nomic-embed-v1.5",  name: "Nomic Embed v1.5",     role: "embedding",
        sizeGB: 0.3,  tag: "v1.5",                recommended: true,
        note: "required · indexes sessions, memories, refs",    pulled: true  },
      { id: "llama-3.1-70b",     name: "Llama 3.1 70B",        role: "inference",
        sizeGB: 42.5, tag: "70b-instruct-q4_K_M", recommended: false,
        note: "heaviest · only if you have the RAM headroom",   pulled: false },
      { id: "qwen-coder-7b",     name: "Qwen2.5-Coder 7B",     role: "voice",
        sizeGB: 4.7,  tag: "7b-instruct-q5_K_M",  recommended: false,
        note: "light · conversational · good for voice",       pulled: false }
    ],
    providers: [
      { id: "anthropic", name: "Anthropic",  kanji: "人",
        kind: "cloud",  envVar: "ANTHROPIC_API_KEY",
        detected: true,  configured: true,
        note: "detected in your shell · highest-quality reasoning",
        models: [
          { id: "claude-sonnet-4.5", name: "Claude Sonnet 4.5",  role: "inference",
            context: "200K", cost: "$3 / 1M in  ·  $15 / 1M out" },
          { id: "claude-opus-4",     name: "Claude Opus 4",      role: "inference",
            context: "200K", cost: "$15 / 1M in ·  $75 / 1M out" },
          { id: "claude-haiku-4",    name: "Claude Haiku 4",     role: "voice",
            context: "200K", cost: "$1 / 1M in  ·  $5 / 1M out"  }
        ]},
      { id: "ollama",    name: "Ollama",     kanji: "羊",
        kind: "local",  envVar: null,
        detected: true,  configured: true,
        note: "running locally · no key needed",
        models: [
          { id: "qwen-coder-32b",   name: "Qwen2.5-Coder 32B",  role: "inference",
            sizeGB: 19.9, pulled: false, recommended: true,
            note: "primary inference · fits 64 GB" },
          { id: "deepseek-r1-14b",  name: "DeepSeek-R1 14B",    role: "consolidation",
            sizeGB: 8.4,  pulled: false, recommended: true,
            note: "fast reasoning · memory consolidation" },
          { id: "nomic-embed-v1.5", name: "Nomic Embed v1.5",   role: "embedding",
            sizeGB: 0.3,  pulled: true,  recommended: true,
            note: "required · indexes sessions & memories" },
          { id: "qwen-coder-7b",    name: "Qwen2.5-Coder 7B",   role: "voice",
            sizeGB: 4.7,  pulled: false, recommended: false,
            note: "light · conversational · good for voice" }
        ]},
      { id: "openai",    name: "OpenAI",     kanji: "開",
        kind: "cloud",  envVar: "OPENAI_API_KEY",
        detected: false, configured: false,
        note: "gpt-4o · gpt-4-turbo · o1",
        models: [
          { id: "gpt-4o",       name: "GPT-4o",       role: "inference",     context: "128K" },
          { id: "gpt-4o-mini",  name: "GPT-4o mini",  role: "consolidation", context: "128K" },
          { id: "o1",           name: "o1",           role: "inference",     context: "128K" }
        ]},
      { id: "google",    name: "Google",     kanji: "星",
        kind: "cloud",  envVar: "GEMINI_API_KEY",
        detected: false, configured: false,
        note: "gemini-2.5-pro · gemini-flash",
        models: [
          { id: "gemini-2.5-pro",   name: "Gemini 2.5 Pro",   role: "inference",     context: "2M" },
          { id: "gemini-flash",     name: "Gemini Flash",     role: "voice",         context: "1M" }
        ]}
    ],

    // Priority — which model handles each role. First item is primary;
    // subsequent items are fallbacks used when primary fails or is rate-limited.
    rolePriority: {
      inference:     ["claude-sonnet-4.5", "qwen-coder-32b", "gpt-4o"],
      consolidation: ["deepseek-r1-14b"],
      embedding:     ["nomic-embed-v1.5"],
      voice:         ["claude-haiku-4", "qwen-coder-7b"],
      fallback:      ["qwen-coder-32b"]
    },

    // Providers sensei knows about but the user hasn't added yet
    addable: [
      { id: "openai",  name: "OpenAI",     kanji: "開", kind: "cloud" },
      { id: "google",  name: "Google",     kanji: "星", kind: "cloud" },
      { id: "groq",    name: "Groq",       kanji: "速", kind: "cloud" },
      { id: "xai",     name: "xAI",        kanji: "乂", kind: "cloud" },
      { id: "custom",  name: "Custom · OpenAI-compatible", kanji: "自", kind: "custom" }
    ]
  }
};
