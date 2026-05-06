// MCP Signals — mock data for Replay and Insights views.
//
// Replay: per-session timeline of MCP tool calls (what the assistant asked,
// what it received, what it did with it).
//
// Insights: aggregated usage + effectiveness metrics across sessions.
//
// Data is illustrative. Numbers are tuned to produce interesting chart shapes
// and realistic-looking signals, not to be self-consistent with other mocks.

window.MCP_SIGNALS = {
  // ─── per-session tool-call timelines ─────────────────────────────
  // Keyed by session id (matches window.SENSEI_DATA.sessions ids).
  //
  // Each call captures: the tool, the exact args, a short response excerpt,
  // duration, and whether the assistant USED the response in its next action.
  //   usage: "used"     — the next edit/action references returned data
  //          "partial"  — some of the data used, some ignored
  //          "ignored"  — response came back clean but assistant didn't act on it
  sessions: {
    "s-2891": {
      title: "Fix refresh token rotation edge case",
      project: "lumen-auth",
      ftr: false,
      totalTurns: 14,
      corrections: 3,
      toolCallCount: 9,
      calls: [
        { i: 1, t: "10:42", turn: 1,
          tool: "sensei.project.explain-structure",
          args: { project: "lumen-auth" },
          durationMs: 340,
          responseSnippet: "lumen-auth is a Rust service … publishes SessionEvent, depends on lumen-api core types.",
          usage: "used", note: "cited service boundary in turn 2" },
        { i: 2, t: "10:43", turn: 2,
          tool: "sensei.library.search-usages",
          args: { library: "tokio", query: "timeout" },
          durationMs: 412,
          responseSnippet: "12 call sites · patterns: tokio::time::timeout + tokio::select! race",
          usage: "used", note: "referenced timeout pattern" },
        { i: 3, t: "10:44", turn: 2,
          tool: "sensei.project.find-hotspots",
          args: { project: "lumen-auth", since: "7d" },
          durationMs: 1120,
          responseSnippet: "refresh.ts · 8 edits · 3 rework sessions · god-node candidate",
          usage: "used", note: "mentioned as rework file" },
        { i: 4, t: "10:46", turn: 3,
          tool: "sensei.session.search",
          args: { query: "refresh token", limit: 5 },
          durationMs: 220,
          responseSnippet: "3 prior sessions · s-2720 (FTR), s-2543 (corrected), s-2321 (corrected)",
          usage: "partial", note: "cited s-2720 but missed s-2543's correction" },
        { i: 5, t: "10:49", turn: 4,
          tool: "sensei.library.get-callers",
          args: { library: "jsonwebtoken", symbol: "decode" },
          durationMs: 510,
          responseSnippet: "4 callers · refresh.ts:88, middleware.ts:23, …",
          usage: "ignored", note: "result never referenced in subsequent turns" },
        { i: 6, t: "10:53", turn: 6,
          tool: "sensei.pattern.match",
          args: { project: "lumen-auth", file: "src/auth/refresh.ts" },
          durationMs: 680,
          responseSnippet: "'Retry with exponential backoff' · emerging · 3 nearby uses",
          usage: "used", note: "applied pattern in next edit" },
        { i: 7, t: "11:02", turn: 9,
          tool: "sensei.session.outcome-of",
          args: { session: "s-2720" },
          durationMs: 180,
          responseSnippet: "FTR · 22m · added skewTolerance · no corrections",
          usage: "used", note: "adopted the skewTolerance approach" },
        { i: 8, t: "11:09", turn: 11,
          tool: "sensei.library.check-drift",
          args: { library: "jsonwebtoken", project: "lumen-auth" },
          durationMs: 920,
          responseSnippet: "no drift · 0 deviations",
          usage: "ignored", note: "result was clean, no action needed — but no ack" },
        { i: 9, t: "11:16", turn: 13,
          tool: "sensei.project.validate",
          args: { project: "lumen-auth" },
          durationMs: 1540,
          responseSnippet: "lint ✓ · tests 5/5 · 0 clippy warnings",
          usage: "used", note: "closed the loop before commit" }
      ]
    },

    "s-2890": {
      title: "Add bezier smoothing to freehand tool",
      project: "lumen-canvas",
      ftr: true,
      totalTurns: 6,
      corrections: 0,
      toolCallCount: 4,
      calls: [
        { i: 1, t: "09:15", turn: 1,
          tool: "sensei.project.explain-structure",
          args: { project: "lumen-canvas" },
          durationMs: 290,
          responseSnippet: "single-repo TS · canvas/tools is the render pipeline",
          usage: "used" },
        { i: 2, t: "09:16", turn: 1,
          tool: "sensei.pattern.match",
          args: { project: "lumen-canvas", file: "src/tools/freehand.ts" },
          durationMs: 560,
          responseSnippet: "'Stroke smoothing' pattern · canonical · 2 existing uses",
          usage: "used", note: "mirrored existing strategy" },
        { i: 3, t: "09:22", turn: 3,
          tool: "sensei.library.search-usages",
          args: { library: "d3-interpolate", query: "catmullRom" },
          durationMs: 310,
          responseSnippet: "0 results · not a dependency",
          usage: "used", note: "pivoted to built-in bezier helper" },
        { i: 4, t: "09:34", turn: 5,
          tool: "sensei.project.validate",
          args: { project: "lumen-canvas" },
          durationMs: 1180,
          responseSnippet: "lint ✓ · tests 3/3 · 0 type errors",
          usage: "used" }
      ]
    },

    "s-2889": {
      title: "Add OAuth device flow",
      project: "lumen-auth",
      ftr: false,
      totalTurns: 22,
      corrections: 4,
      toolCallCount: 13,
      calls: [
        { i: 1, t: "16:02", turn: 1,
          tool: "sensei.project.explain-structure",
          args: { project: "lumen-auth" },
          durationMs: 320,
          responseSnippet: "lumen-auth is a Rust service …",
          usage: "used" },
        { i: 2, t: "16:03", turn: 1,
          tool: "sensei.session.search",
          args: { query: "oauth device flow", limit: 5 },
          durationMs: 195,
          responseSnippet: "0 results",
          usage: "used" },
        { i: 3, t: "16:05", turn: 2,
          tool: "sensei.pattern.match",
          args: { project: "lumen-auth", file: "src/oauth" },
          durationMs: 620,
          responseSnippet: "no matching pattern · no nearby uses",
          usage: "ignored", note: "invented an approach instead of asking for examples" },
        { i: 4, t: "16:12", turn: 4,
          tool: "sensei.library.search-usages",
          args: { library: "reqwest", query: "poll" },
          durationMs: 410,
          responseSnippet: "2 call sites · both in api/, not auth/",
          usage: "partial" },
        { i: 5, t: "16:22", turn: 7,
          tool: "sensei.library.get-callers",
          args: { library: "tower-http", symbol: "Trace" },
          durationMs: 380,
          responseSnippet: "1 caller · middleware.rs:12",
          usage: "ignored" },
        { i: 6, t: "16:34", turn: 10,
          tool: "sensei.project.validate",
          args: { project: "lumen-auth" },
          durationMs: 1620,
          responseSnippet: "lint ✗ · 3 clippy warnings · tests 0/4",
          usage: "used" },
        { i: 7, t: "16:48", turn: 13,
          tool: "sensei.session.outcome-of",
          args: { session: "s-2321" },
          durationMs: 190,
          responseSnippet: "corrected · 'integration tests for device flow need mock RFC 8628 server'",
          usage: "used", note: "finally added the integration pattern" },
        { i: 8, t: "17:01", turn: 16,
          tool: "sensei.project.validate",
          args: { project: "lumen-auth" },
          durationMs: 1710,
          responseSnippet: "lint ✓ · tests 3/4 · 1 failing",
          usage: "used" },
        { i: 9, t: "17:08", turn: 18,
          tool: "sensei.library.check-drift",
          args: { library: "oauth2", project: "lumen-auth" },
          durationMs: 720,
          responseSnippet: "no drift",
          usage: "ignored" },
        { i: 10, t: "17:14", turn: 21,
          tool: "sensei.project.validate",
          args: { project: "lumen-auth" },
          durationMs: 1580,
          responseSnippet: "lint ✓ · tests 4/4",
          usage: "used" }
      ]
    }
  },

  // ─── aggregated insights ──────────────────────────────────────────
  // All numbers are illustrative. Sparkline data is 14 daily buckets,
  // oldest-first.
  insights: {
    window: "30d",
    sessionsAnalyzed: 142,

    // Per-tool usage rollup.
    // adoption signals: calls over window, trend 14d, last 24h
    // effectiveness signals: usage split, ftr-delta (FTR rate when tool used
    // vs. when not), and a short diagnosis when something looks off.
    toolUsage: [
      { tool: "sensei.project.explain-structure",     category: "project",
        calls: 412, trend: [24,22,26,30,28,32,30,34,31,29,35,33,38,41],
        last24h: 41, usedPct: 89, partialPct: 9, ignoredPct: 2,
        ftrDelta: +0.07, verdict: "healthy",
        note: "Every skill mentions it first. Effective." },
      { tool: "sensei.library.search-usages",         category: "library",
        calls: 387, trend: [18,22,24,26,28,30,29,32,33,31,34,36,35,39],
        last24h: 39, usedPct: 82, partialPct: 12, ignoredPct: 6,
        ftrDelta: +0.09, verdict: "healthy",
        note: "Single biggest FTR lift — mentioned in 4 personas." },
      { tool: "sensei.pattern.match",                 category: "pattern",
        calls: 261, trend: [12,14,15,18,20,22,25,27,24,26,28,30,29,31],
        last24h: 31, usedPct: 71, partialPct: 19, ignoredPct: 10,
        ftrDelta: +0.12, verdict: "healthy",
        note: "After 'emerging patterns' persona added, FTR gap widened." },
      { tool: "sensei.project.validate",              category: "project",
        calls: 224, trend: [10,11,12,14,13,16,17,18,19,21,22,23,25,27],
        last24h: 27, usedPct: 95, partialPct: 4, ignoredPct: 1,
        ftrDelta: +0.04, verdict: "healthy",
        note: "Closing-turn tool. High usage, small FTR lift (tautological)." },
      { tool: "sensei.library.get-callers",           category: "library",
        calls: 189, trend: [16,15,18,17,19,20,18,17,19,16,14,13,12,10],
        last24h: 10, usedPct: 48, partialPct: 22, ignoredPct: 30,
        ftrDelta: -0.02, verdict: "warn",
        note: "30% of calls go ignored — response format (nested tree) may be hard to parse." },
      { tool: "sensei.session.search",                category: "session",
        calls: 174, trend: [8,10,11,13,12,14,13,15,14,16,15,14,12,11],
        last24h: 11, usedPct: 64, partialPct: 24, ignoredPct: 12,
        ftrDelta: +0.06, verdict: "healthy",
        note: "Fine — assistants use it to avoid duplicate work." },
      { tool: "sensei.session.outcome-of",            category: "session",
        calls: 96,  trend: [4,5,5,6,7,7,8,8,9,9,10,11,12,12],
        last24h: 12, usedPct: 83, partialPct: 11, ignoredPct: 6,
        ftrDelta: +0.11, verdict: "healthy",
        note: "High-leverage when used. Consider surfacing more prominently." },
      { tool: "sensei.project.find-hotspots",         category: "project",
        calls: 71,  trend: [3,4,4,5,5,6,5,6,7,6,7,8,7,8],
        last24h: 8, usedPct: 58, partialPct: 26, ignoredPct: 16,
        ftrDelta: +0.03, verdict: "ok",
        note: "Niche — mostly used at session start for architectural work." },
      { tool: "sensei.library.check-drift",           category: "library",
        calls: 38,  trend: [1,2,1,2,2,3,2,3,2,3,3,4,3,4],
        last24h: 4, usedPct: 22, partialPct: 18, ignoredPct: 60,
        ftrDelta: -0.01, verdict: "warn",
        note: "60% ignored. Clean responses give no signal. Consider auto-call only when drift detected." },
      { tool: "sensei.project.find-duplication",      category: "project",
        calls: 12,  trend: [1,0,1,1,0,1,1,0,1,1,0,1,2,1],
        last24h: 1, usedPct: 67, partialPct: 17, ignoredPct: 16,
        ftrDelta: +0.08, verdict: "ok",
        note: "Low adoption despite good outcomes. Skill doesn't instruct it." },
      { tool: "sensei.pattern.propose",               category: "pattern",
        calls: 4,   trend: [0,0,0,1,0,0,0,0,1,0,0,1,0,1],
        last24h: 1, usedPct: 100, partialPct: 0, ignoredPct: 0,
        ftrDelta: 0,    verdict: "underused",
        note: "Registered but effectively never called. Skill doesn't mention pattern proposal." },
      { tool: "sensei.session.recommend-next",        category: "session",
        calls: 0,   trend: [0,0,0,0,0,0,0,0,0,0,0,0,0,0],
        last24h: 0, usedPct: 0,  partialPct: 0, ignoredPct: 0,
        ftrDelta: 0,    verdict: "unused",
        note: "Zero calls in 30d. Either undiscoverable or the skill contradicts it." }
    ],

    // By-project usage — which projects lean on which tools
    byProject: [
      { project: "lumen-auth",    sessions: 38, toolCalls: 287, topTool: "sensei.library.search-usages",  ftr: 0.52 },
      { project: "lumen-canvas",  sessions: 29, toolCalls: 142, topTool: "sensei.pattern.match",          ftr: 0.76 },
      { project: "lumen-api",     sessions: 26, toolCalls: 198, topTool: "sensei.project.validate",       ftr: 0.65 },
      { project: "brand-tokens",  sessions: 12, toolCalls: 38,  topTool: "sensei.project.explain-structure", ftr: 0.83 },
      { project: "lumen-sync",    sessions: 21, toolCalls: 176, topTool: "sensei.library.search-usages",  ftr: 0.58 },
      { project: "lumen-app",     sessions: 16, toolCalls: 87,  topTool: "sensei.pattern.match",          ftr: 0.72 }
    ],

    // Skill/command tuning signals — the "what SHOULD we change?" bullets
    signals: [
      { kind: "warn", kanji: "警",
        title: "check-drift: 60% ignored",
        body: "Clean responses produce no observable action. Consider moving drift detection into a passive rule that auto-fires only when drift is detected, instead of asking the assistant to poll.",
        action: "Edit rule: libraries/auto-drift" },
      { kind: "warn", kanji: "警",
        title: "get-callers response often ignored",
        body: "30% of calls show no follow-up action. The nested-tree response may be hard to parse mid-turn. Try a flatter list with call-site snippets.",
        action: "Edit tool: sensei.library.get-callers" },
      { kind: "opportunity", kanji: "機",
        title: "find-duplication is high-leverage but underused",
        body: "67% use rate when called, but only 12 calls in 30 days. No persona mentions duplication hunting. Adding it to the refactor persona could unlock wins.",
        action: "Edit persona: refactor" },
      { kind: "unused", kanji: "眠",
        title: "recommend-next: 0 calls in 30d",
        body: "Tool is registered and valid but never called. Either the skill doesn't instruct it, the name is unclear, or another tool is winning the slot.",
        action: "Trace: why is this dormant?" },
      { kind: "win", kanji: "勝",
        title: "Persona change → FTR up 14%",
        body: "After adding the auth persona (3d ago), search-usages calls in lumen-auth sessions dropped 40% — because the persona already provides the right call sites. FTR rose 14%.",
        action: "Apply same pattern to lumen-sync?" }
    ],

    // Overall per-window deltas shown in the strip at the top of Insights
    deltas: {
      ftrThisWindow: 0.63,   // 63% FTR over the 30-day window
      ftrTrend: +0.08,        // +8 pts vs previous window
      totalCalls: 1868,
      totalCallsTrend: +0.12, // +12% vs previous window
      unusedTools: 1,
      warnTools: 2
    }
  }
};
