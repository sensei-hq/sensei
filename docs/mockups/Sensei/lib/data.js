// Shared fake data for Sensei observatory — Lumen design tool company
// All three directions read from this so the underlying story is identical.

window.SENSEI_DATA = {
  user: { name: "Aiko", handle: "aiko@lumen.design" },

  solutions: [
    {
      id: "lumen-studio",
      name: "Lumen Studio",
      kanji: "工",
      repos: ["lumen-app", "lumen-canvas", "lumen-shell"],
      ftr: 0.82,
      ftrPrev: 0.74,
      sessions7d: 41,
      tokens7d: 2.4,
      activeSkills: 9,
      description: "Desktop design tool · Rust + Svelte",
      focus: "canvas engine"
    },
    {
      id: "lumen-cloud",
      name: "Lumen Cloud",
      kanji: "雲",
      repos: ["lumen-api", "lumen-sync", "lumen-auth"],
      ftr: 0.64,
      ftrPrev: 0.71,
      sessions7d: 28,
      tokens7d: 1.8,
      activeSkills: 6,
      description: "Sync & collaboration backend",
      focus: "auth module",
      warning: true
    },
    {
      id: "brand-kit",
      name: "Brand Kit",
      kanji: "紋",
      repos: ["brand-tokens", "brand-docs"],
      ftr: 0.91,
      ftrPrev: 0.88,
      sessions7d: 12,
      tokens7d: 0.4,
      activeSkills: 4,
      description: "Design system & tokens library",
      focus: "token generation"
    }
  ],

  // Global FTR sparkline — 14 days
  ftrHistory: [
    0.71, 0.69, 0.74, 0.72, 0.68, 0.70, 0.73,
    0.75, 0.72, 0.78, 0.74, 0.79, 0.76, 0.78
  ],

  // Per-solution 14d sparkline
  ftrBySolution: {
    "lumen-studio": [0.72, 0.74, 0.71, 0.76, 0.78, 0.75, 0.79, 0.77, 0.80, 0.82, 0.78, 0.81, 0.80, 0.82],
    "lumen-cloud":  [0.71, 0.73, 0.70, 0.68, 0.65, 0.69, 0.67, 0.64, 0.62, 0.66, 0.63, 0.61, 0.65, 0.64],
    "brand-kit":    [0.88, 0.89, 0.87, 0.90, 0.88, 0.91, 0.90, 0.92, 0.91, 0.93, 0.90, 0.91, 0.92, 0.91]
  },

  sessions: [
    {
      id: "s-2891",
      project: "lumen-auth",
      solution: "lumen-cloud",
      title: "Fix refresh token rotation edge case",
      started: "10:42",
      date: "Today",
      duration: "38m",
      turns: 14,
      tokens: 41200,
      ftr: false,
      corrections: 3,
      module: "auth",
      outcome: "corrected",
      summary: "AI missed the clock-skew tolerance required by the legacy SDK. Corrected after third turn.",
      events: [
        { t: "10:42", kind: "start", text: "session begins · claude-code · cwd lumen-auth" },
        { t: "10:43", kind: "context", text: "loaded 4 files · session-start rules v2.1" },
        { t: "10:45", kind: "edit", text: "edited src/auth/refresh.ts" },
        { t: "10:51", kind: "test", text: "test failed — TokenExpiredError at offset +3s" },
        { t: "10:53", kind: "correction", text: "dev: 'account for 30s clock skew per SDK 4.2'" },
        { t: "10:58", kind: "edit", text: "edited src/auth/refresh.ts · added skewTolerance" },
        { t: "11:02", kind: "test", text: "5/5 passing" },
        { t: "11:08", kind: "correction", text: "dev: 'also handle the case where refresh arrives during rotation'" },
        { t: "11:14", kind: "edit", text: "edited src/auth/refresh.ts · added inFlightMutex" },
        { t: "11:20", kind: "end", text: "committed · 38m · 3 corrections" }
      ]
    },
    {
      id: "s-2890",
      project: "lumen-canvas",
      solution: "lumen-studio",
      title: "Add bezier smoothing to freehand tool",
      started: "09:15",
      date: "Today",
      duration: "22m",
      turns: 6,
      tokens: 18400,
      ftr: true,
      corrections: 0,
      module: "canvas/tools",
      outcome: "first-try",
      summary: "Pattern matched existing smoothing strategy. Tests passed first run.",
      events: [
        { t: "09:15", kind: "start", text: "session begins" },
        { t: "09:16", kind: "context", text: "loaded canvas-tools persona · 6 files" },
        { t: "09:22", kind: "edit", text: "added smoothBezier in tools/freehand.ts" },
        { t: "09:28", kind: "test", text: "3/3 passing" },
        { t: "09:37", kind: "end", text: "committed · 22m · first try" }
      ]
    },
    {
      id: "s-2889",
      project: "lumen-auth",
      solution: "lumen-cloud",
      title: "Add OAuth device flow",
      started: "Yesterday 16:02",
      date: "Yesterday",
      duration: "1h 12m",
      turns: 22,
      tokens: 68300,
      ftr: false,
      corrections: 4,
      module: "auth",
      outcome: "corrected",
      summary: "No integration-test pattern for device flow. AI invented one that contradicted house style."
    },
    {
      id: "s-2888",
      project: "brand-tokens",
      solution: "brand-kit",
      title: "Generate dark-mode color ramps",
      started: "Yesterday 14:30",
      date: "Yesterday",
      duration: "18m",
      turns: 5,
      tokens: 14100,
      ftr: true,
      corrections: 0,
      module: "tokens/color",
      outcome: "first-try",
      summary: "Token rules crystal clear. One-shot."
    },
    {
      id: "s-2887",
      project: "lumen-app",
      solution: "lumen-studio",
      title: "Refactor layer panel drag-drop",
      started: "Yesterday 11:18",
      date: "Yesterday",
      duration: "54m",
      turns: 11,
      tokens: 32800,
      ftr: true,
      corrections: 0,
      module: "panels",
      outcome: "first-try",
      summary: "Clean."
    },
    {
      id: "s-2886",
      project: "lumen-auth",
      solution: "lumen-cloud",
      title: "Session invalidation on password change",
      started: "2d ago 17:44",
      date: "2d ago",
      duration: "44m",
      turns: 9,
      tokens: 26700,
      ftr: false,
      corrections: 2,
      module: "auth",
      outcome: "corrected",
      summary: "Missed the cache-invalidation pattern we use everywhere else."
    },
    {
      id: "s-2885",
      project: "lumen-sync",
      solution: "lumen-cloud",
      title: "CRDT merge on offline reconnect",
      started: "2d ago 10:01",
      date: "2d ago",
      duration: "1h 30m",
      turns: 18,
      tokens: 54200,
      ftr: true,
      corrections: 0,
      module: "sync/crdt",
      outcome: "first-try"
    },
    {
      id: "s-2884",
      project: "lumen-canvas",
      solution: "lumen-studio",
      title: "Snap-to-grid toggle",
      started: "3d ago",
      date: "3d ago",
      duration: "14m",
      turns: 4,
      tokens: 9800,
      ftr: true,
      corrections: 0,
      module: "canvas/tools"
    }
  ],

  // Coaching recommendations — the "sensei speaks" part
  coaching: [
    {
      id: "c1",
      urgency: "high",
      koan: "The AI does not know your auth.",
      body: "Three sessions corrected in lumen-auth this week. All touched refresh or device flow. There is no integration-test persona for this module.",
      action: "Add auth persona",
      actionDetail: "Extract from s-2891 and s-2889",
      impact: "Projected FTR +14% in lumen-cloud",
      module: "auth"
    },
    {
      id: "c2",
      urgency: "medium",
      koan: "Canvas teaches clearly. Listen.",
      body: "lumen-canvas sessions average 92% FTR. The `tools/` folder holds two patterns the AI consistently matches. Promote them to solution rules — other repos will benefit.",
      action: "Promote patterns",
      actionDetail: "smoothBezier · snapToGrid",
      impact: "Share across lumen-studio",
      module: "canvas/tools"
    },
    {
      id: "c3",
      urgency: "low",
      koan: "Stale docs mislead the student.",
      body: "brand-tokens README last touched 47 days ago. Three public APIs have drifted. Enable drift-detection skill for brand-kit.",
      action: "Enable skill",
      actionDetail: "doc-drift · brand-kit",
      impact: "Prevents coming confusion",
      module: "docs"
    }
  ],

  // Complexity / god-nodes
  hotspots: [
    { name: "lumen-api/src/router.ts",     fanIn: 42, fanOut: 18, rework: 7, severity: "god" },
    { name: "lumen-auth/src/session.ts",   fanIn: 28, fanOut: 12, rework: 5, severity: "god" },
    { name: "lumen-canvas/src/engine.ts",  fanIn: 19, fanOut: 22, rework: 2, severity: "cluster" },
    { name: "lumen-sync/src/crdt.ts",      fanIn: 14, fanOut: 11, rework: 1, severity: "ok" }
  ],

  skills: [
    { id: "test-gen",       name: "Test generation",        active: true,  solutions: ["lumen-studio","lumen-cloud"] },
    { id: "doc-drift",      name: "Doc drift detection",    active: false, solutions: [] },
    { id: "pattern-extract",name: "Pattern extraction",     active: true,  solutions: ["lumen-studio","lumen-cloud","brand-kit"] },
    { id: "context-pack",   name: "Context pack tuning",    active: true,  solutions: ["lumen-studio"] },
    { id: "session-coach",  name: "Session coaching",       active: true,  solutions: ["lumen-cloud"] },
    { id: "graph-index",    name: "Graph indexer",          active: true,  solutions: ["lumen-studio","lumen-cloud","brand-kit"] },
    { id: "refactor-target",name: "Refactor target warn",   active: false, solutions: [] },
    { id: "lib-docs",       name: "Library docs (MCP)",     active: true,  solutions: ["lumen-studio","lumen-cloud"] }
  ],

  personas: [
    { id: "canvas-tools",  name: "Canvas tools",  solution: "lumen-studio", triggers: "cwd matches lumen-canvas/src/tools" },
    { id: "layer-panel",   name: "Layer panel",   solution: "lumen-studio", triggers: "cwd matches lumen-app/src/panels" },
    { id: "sync-crdt",     name: "Sync CRDT",     solution: "lumen-cloud",  triggers: "cwd matches lumen-sync/src/crdt" },
    { id: "token-generator",name:"Token generator",solution:"brand-kit",    triggers: "cwd matches brand-tokens/src" }
  ],

  libraries: [
    { name: "Svelte 5",       version: "5.0.0",  pages: 342, lastIndexed: "2d ago" },
    { name: "Tauri",          version: "2.1.0",  pages: 128, lastIndexed: "5d ago" },
    { name: "Rust std",       version: "1.85",   pages: 891, lastIndexed: "1w ago" },
    { name: "SQLx",           version: "0.8",    pages: 64,  lastIndexed: "3d ago" }
  ]
};
