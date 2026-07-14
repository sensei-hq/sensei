// ═══════════════════════════════════════════════════════════════════
//  SENSEI · SOLUTION TRACK
//  A "solution" is a client engagement that bundles several projects
//  (repos) into one thing sensei reasons about as a whole. Three
//  screens, one window with a segmented nav — mirrors DojoConsole's
//  `initial` pattern:
//    · Dashboard    — aggregate FTR strip + per-project rollup
//    · Architecture — the MERGED code graph across every repo
//                     (cross-project edges dashed), built on AtlasGraph
//    · Sessions     — every session in the solution, filterable by
//                     project (empty state included)
//  Self-contained data; reuses window.AtlasGraph, window.AtlasLegend,
//  window.ObsFtrStrip when present. Token-only colors → theme-free.
// ═══════════════════════════════════════════════════════════════════

const { useState: solS } = React;

// ─── The engagement ─────────────────────────────────────────
const SOL = {
  name: "Lumen Cloud",
  client: "lumen-systems",
  kanji: "束",                        // 束 — a bundle
  projects: [
    { id: "auth",   kanji: "鍵", name: "lumen-auth",   ftr: 0.64, sessions7d: 28, warn: true,
      trend: [0.61,0.58,0.63,0.60,0.57,0.62,0.64,0.61,0.66,0.63,0.60,0.67,0.65,0.64] },
    { id: "canvas", kanji: "筆", name: "lumen-canvas", ftr: 0.82, sessions7d: 41, warn: false,
      trend: [0.78,0.80,0.79,0.83,0.81,0.84,0.82,0.85,0.83,0.86,0.84,0.87,0.85,0.82] },
    { id: "shared", kanji: "核", name: "shared-core",  ftr: 0.88, sessions7d: 12, warn: false,
      trend: [0.84,0.86,0.85,0.88,0.87,0.89,0.88,0.90,0.89,0.91,0.90,0.92,0.90,0.88] },
  ],

  // Aggregate FTR trend for the whole solution (14d)
  trend14: [0.70,0.69,0.72,0.71,0.68,0.72,0.74,0.73,0.76,0.74,0.72,0.77,0.75,0.75],
  ftr: 0.75, delta: +0.05,

  // Rollup insights — things true across the engagement, never more than 3
  insights: [
    { kanji: "繰", label: "Pattern recurring", project: "auth",
      text: "Refresh-token rotation corrected again in lumen-auth.", tag: "3rd time", tone: "warn" },
    { kanji: "昇", label: "Teaching travelled", project: "canvas",
      text: "Bezier smoothing rule promoted in lumen-canvas — adopted by shared-core.", tag: "+7% FTR", tone: "good" },
    { kanji: "探", label: "Boundary drift", project: "shared",
      text: "brand-tokens API drifted from two consumers across projects.", tag: "low urgency", tone: "mute" },
  ],

  sessions: [
    { id: "s-2891", project: "auth",   title: "Fix refresh token rotation", time: "10:42",     duration: "38m",    ftr: false, corrections: 3 },
    { id: "s-2890", project: "canvas", title: "Bezier smoothing tool",       time: "09:15",     duration: "22m",    ftr: true,  corrections: 0 },
    { id: "s-2889", project: "auth",   title: "OAuth device flow",           time: "Yesterday", duration: "1h 12m", ftr: false, corrections: 4 },
    { id: "s-2888", project: "shared", title: "Dark-mode color ramps",       time: "Yesterday", duration: "18m",    ftr: true,  corrections: 0 },
    { id: "s-2885", project: "canvas", title: "Layer panel drag reorder",    time: "Mon",       duration: "51m",    ftr: true,  corrections: 1 },
    { id: "s-2882", project: "auth",   title: "Session clock-skew tolerance",time: "Mon",       duration: "27m",    ftr: true,  corrections: 0 },
  ],
};

// The merged architecture graph: repos from every project on one canvas.
// node.repo = project id (drives the project focus/dimming in AtlasGraph);
// boundary edges are cross-project dependencies (rendered dashed).
const SOL_GRAPH = {
  nodes: [
    { id: "auth-svc", kind: "repo",    repo: "auth",   kanji: "鍵", label: "lumen-auth",     sub: "Rust · axum",    x: 175, y: 150, r: 33 },
    { id: "auth-ui",  kind: "package", repo: "auth",   kanji: "客", label: "@lumen/auth-ui", sub: "React",          x: 150, y: 340, r: 27 },
    { id: "canvas",   kind: "repo",    repo: "canvas", kanji: "筆", label: "lumen-canvas",   sub: "TS · editor",    x: 410, y: 120, r: 33 },
    { id: "render",   kind: "package", repo: "canvas", kanji: "描", label: "@lumen/render",  sub: "WebGL",          x: 420, y: 305, r: 27 },
    { id: "core",     kind: "crate",   repo: "shared", kanji: "核", label: "lumen-core",     sub: "shared crate",   x: 620, y: 215, r: 31 },
    { id: "tokens",   kind: "package", repo: "shared", kanji: "紋", label: "brand-tokens",   sub: "design tokens",  x: 645, y: 405, r: 26 },
  ],
  edges: [
    { from: "auth-ui", to: "auth-svc" },
    { from: "canvas",  to: "render"   },
    { from: "auth-svc", to: "core",  boundary: true },
    { from: "render",   to: "core",  boundary: true },
    { from: "canvas",   to: "tokens", boundary: true },
    { from: "auth-ui",  to: "tokens", boundary: true },
  ],
};

const projName = id => (SOL.projects.find(p => p.id === id) || { name: id }).name;
const projKanji = id => (SOL.projects.find(p => p.id === id) || { kanji: "○" }).kanji;
const toneColor = t => t === "warn" ? "var(--warning)" : t === "good" ? "var(--success)" : "var(--ink-3)";

// ═══════════════════════════════════════════════════════════════════
//  Window chrome + nav shell
// ═══════════════════════════════════════════════════════════════════
const SOL_NAV = [
  { id: "dashboard",    label: "Dashboard"    },
  { id: "architecture", label: "Architecture" },
  { id: "sessions",     label: "Sessions"     },
];

function SolChrome({ view }) {
  return (
    <div style={{ height: 38, background: "var(--paper-2)", borderBottom: "var(--hairline)",
                  display: "flex", alignItems: "center", flexShrink: 0, position: "relative" }}>
      <div style={{ display: "flex", position: "absolute", left: 14 }} className="gap-2">
        {["#ED6A5E", "#F4BF4F", "#61C554"].map(c => (
          <span key={c} style={{ width: 11, height: 11, borderRadius: "50%", background: c }}/>
        ))}
      </div>
      <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center",
                    justifyContent: "center", pointerEvents: "none" }}>
        <span style={{ fontSize: 12, color: "var(--ink-2)" }}>
          <span className="kanji" style={{ color: "var(--accent)" }}>先生</span>
          {"  ·  Solution · " + SOL.name}
          <span style={{ color: "var(--ink-4)" }}>{"  ·  " + view}</span>
        </span>
      </div>
      <div style={{ position: "absolute", top: 0, left: 0, right: 0, height: 2, background: "var(--accent)" }}/>
    </div>
  );
}

// slim left rail: the engagement + its member projects
function SolRail({ view, setView, filter, setFilter }) {
  return (
    <aside style={{ width: 230, borderRight: "var(--hairline)", background: "var(--paper-2)",
                    display: "flex", flexDirection: "column", minHeight: 0 }}>
      <div className="px-5 pt-5 pb-4" style={{ borderBottom: "var(--hairline)" }}>
        <div style={{ display: "flex", alignItems: "center" }} className="gap-3">
          <span className="kanji" style={{ fontSize: 26, color: "var(--accent)", lineHeight: 1 }}>{SOL.kanji}</span>
          <div>
            <div style={{ fontSize: 15, fontWeight: 600, color: "var(--ink)" }}>{SOL.name}</div>
            <div className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{SOL.client}</div>
          </div>
        </div>
      </div>

      <div className="px-3 pt-4" style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        {SOL_NAV.map(n => (
          <button key={n.id} onClick={() => setView(n.id)} style={{
            display: "flex", alignItems: "center", width: "100%", textAlign: "left",
            padding: "8px 12px", borderRadius: 6, border: "none", cursor: "pointer",
            fontSize: 13.5, fontWeight: view === n.id ? 600 : 400,
            background: view === n.id ? "var(--paper-3)" : "transparent",
            color: view === n.id ? "var(--ink)" : "var(--ink-2)",
          }}>{n.label}</button>
        ))}
      </div>

      <div className="px-5 pt-6 pb-2" style={{ fontSize: 11, letterSpacing: "0.18em",
            textTransform: "uppercase", color: "var(--ink-3)" }}>Projects · 3</div>
      <div className="px-3" style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        {SOL.projects.map(p => {
          const on = filter === p.id;
          return (
            <button key={p.id} onClick={() => setFilter && setFilter(on ? "all" : p.id)} style={{
              display: "flex", alignItems: "center", gap: 10, width: "100%", textAlign: "left",
              padding: "8px 12px", borderRadius: 6, border: "none", cursor: "pointer",
              background: on ? "var(--paper-3)" : "transparent",
            }}>
              <span className="kanji" style={{ fontSize: 16, color: p.warn ? "var(--warning)" : "var(--ink-3)", width: 18 }}>{p.kanji}</span>
              <span className="mono" style={{ flex: 1, fontSize: 12.5, color: "var(--ink)" }}>{p.name}</span>
              <span className="mono" style={{ fontSize: 11, color: p.warn ? "var(--warning)" : "var(--ink-3)" }}>{Math.round(p.ftr*100)}</span>
            </button>
          );
        })}
      </div>
      <div style={{ flex: 1 }}/>
      <div className="px-5 py-4" style={{ borderTop: "var(--hairline)", fontSize: 11, color: "var(--ink-4)" }}>
        <span className="mono">先生 · sensei</span> watches all three as one.
      </div>
    </aside>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ① DASHBOARD — aggregate strip + per-project rollup
// ═══════════════════════════════════════════════════════════════════
function MiniStrip({ data, warn }) {
  const w = 118, h = 34, n = data.length, gap = 2, barW = (w - gap*(n-1))/n;
  const col = warn ? "var(--warning)" : "var(--success)";
  return (
    <svg width={w} height={h} style={{ display: "block", overflow: "visible" }}>
      {data.map((v, i) => {
        const bh = Math.max(3, v*h), last = i === n-1;
        return <rect key={i} x={i*(barW+gap)} y={h-bh} width={barW} height={bh}
                     fill={last ? col : "var(--edge)"} opacity={last ? 1 : 0.7}/>;
      })}
    </svg>
  );
}

function SolutionDashboard() {
  const Strip = window.ObsFtrStrip;
  return (
    <div style={{ height: "100%", overflow: "auto", background: "var(--paper)" }}>
      {/* aggregate header strip */}
      <div style={{ borderBottom: "var(--hairline)" }} className="px-7 pt-6 pb-5">
        <div style={{ maxWidth: 1000, display: "flex", alignItems: "flex-end",
                      justifyContent: "space-between", flexWrap: "wrap", gap: 24 }} className="mx-auto">
          <div>
            <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase",
                          color: "var(--ink-3)" }} className="mb-1">Solution · aggregate</div>
            <h1 className="display m-0" style={{ fontSize: 28, fontWeight: 400, letterSpacing: "-0.01em" }}>
              Three projects, one first-try-right.
            </h1>
            <p style={{ fontSize: 14, color: "var(--ink-2)", maxWidth: 460, lineHeight: 1.6 }} className="mt-2 mb-0">
              Sensei rolls the whole engagement into one signal — then shows you which project is pulling it.
            </p>
          </div>
          <div style={{ textAlign: "right" }}>
            <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)" }}>
              First-Try-Right · 14d
            </div>
            <div style={{ display: "flex", alignItems: "baseline", justifyContent: "flex-end" }} className="gap-2 mt-1">
              <span className="display" style={{ fontSize: 40, fontWeight: 400, lineHeight: 1, color: "var(--ink)" }}>
                {Math.round(SOL.ftr*100)}
              </span>
              <span style={{ fontSize: 13, color: "var(--ink-3)" }}>%</span>
              <span className="mono ml-1" style={{ fontSize: 11, color: SOL.delta>=0 ? "var(--success)" : "var(--warning)" }}>
                {SOL.delta>=0 ? "↑" : "↓"} {Math.abs(Math.round(SOL.delta*100))}%
              </span>
            </div>
            <div className="mt-2" style={{ display: "flex", justifyContent: "flex-end" }}>
              {Strip
                ? <Strip data={SOL.trend14} value={SOL.ftr} delta={SOL.delta}/>
                : <MiniStrip data={SOL.trend14}/>}
            </div>
          </div>
        </div>
      </div>

      <div style={{ maxWidth: 1000 }} className="mx-auto px-7 py-7">
        {/* per-project rollup */}
        <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)" }} className="mb-4">
          Per project
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16 }}>
          {SOL.projects.map(p => (
            <div key={p.id} style={{ background: "var(--paper-2)", border: "var(--hairline)",
                  borderRadius: 10, padding: 20 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 9 }}>
                  <span className="kanji" style={{ fontSize: 22, color: p.warn ? "var(--warning)" : "var(--ink-3)" }}>{p.kanji}</span>
                  <span className="mono" style={{ fontSize: 13, color: "var(--ink)" }}>{p.name}</span>
                </span>
                {p.warn && <span style={{ fontSize: 10.5, letterSpacing: ".08em", textTransform: "uppercase",
                        color: "var(--warning)", background: "var(--warning-soft)", padding: "3px 8px", borderRadius: 999 }}>needs eyes</span>}
              </div>
              <div style={{ display: "flex", alignItems: "baseline", gap: 4 }} className="mt-4">
                <span className="display" style={{ fontSize: 32, fontWeight: 400, lineHeight: 1,
                      color: p.warn ? "var(--warning)" : "var(--ink)" }}>{Math.round(p.ftr*100)}</span>
                <span style={{ fontSize: 12, color: "var(--ink-3)" }}>% FTR</span>
              </div>
              <div className="mt-3"><MiniStrip data={p.trend} warn={p.warn}/></div>
              <div className="mono mt-3" style={{ fontSize: 11, color: "var(--ink-3)" }}>{p.sessions7d} sessions · 7d</div>
            </div>
          ))}
        </div>

        {/* rollup insights */}
        <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)" }} className="mt-7 mb-3">
          Across the solution
        </div>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 10 }}>
          {SOL.insights.map((x, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center",
                  gap: 14, padding: "14px 18px", borderBottom: i < SOL.insights.length-1 ? "var(--hairline)" : "none" }}>
              <span className="kanji" style={{ fontSize: 22, color: toneColor(x.tone), width: 26 }}>{x.kanji}</span>
              <div>
                <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>
                  {x.label} <span style={{ color: "var(--ink-4)" }}>· {projName(x.project)}</span>
                </div>
                <div style={{ fontSize: 14, color: "var(--ink)", marginTop: 3 }}>{x.text}</div>
              </div>
              <span className="mono" style={{ fontSize: 12, fontWeight: 600, color: toneColor(x.tone), whiteSpace: "nowrap" }}>{x.tag}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ② ARCHITECTURE — the merged code graph across every repo
// ═══════════════════════════════════════════════════════════════════
function SolutionArchitecture({ filter, setFilter }) {
  const [selected, setSelected] = solS(null);
  const Graph = window.AtlasGraph;
  const focus = filter === "all" ? null : filter;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--paper)" }}>
      <div style={{ borderBottom: "var(--hairline)", flexShrink: 0 }} className="px-7 pt-5 pb-4">
        <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", flexWrap: "wrap", gap: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <span className="kanji" style={{ fontSize: 30, color: "var(--accent)", lineHeight: 1 }}>図</span>
            <div>
              <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)" }}>Solution · Architecture</div>
              <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400, letterSpacing: "-0.01em" }}>Merged code graph</h1>
            </div>
          </div>
          {/* project filter chips */}
          <div style={{ display: "flex", background: "var(--paper-3)", borderRadius: 7, padding: 3, gap: 2 }}>
            {[{ id: "all", label: "All projects" }, ...SOL.projects.map(p => ({ id: p.id, label: p.name }))].map(o => {
              const on = filter === o.id;
              return (
                <button key={o.id} onClick={() => { setFilter(o.id); setSelected(null); }} style={{
                  border: "none", cursor: "pointer", borderRadius: 5, padding: "5px 12px",
                  fontSize: 12, fontWeight: on ? 600 : 400,
                  background: on ? "var(--paper)" : "transparent",
                  color: on ? "var(--ink)" : "var(--ink-3)",
                  boxShadow: on ? "0 1px 2px rgba(0,0,0,0.08)" : "none",
                  fontFamily: o.id === "all" ? "var(--font-ui)" : "var(--font-mono)",
                }}>{o.label}</button>
              );
            })}
          </div>
        </div>
        <div style={{ fontSize: 13, color: "var(--ink-2)", marginTop: 12, maxWidth: 640 }}>
          Every repo in the engagement on one canvas. <span style={{ color: "var(--ink-3)" }}>Dashed edges cross a project boundary</span> — the seams sensei watches most closely.
        </div>
      </div>

      <div style={{ flex: 1, position: "relative", minHeight: 0, padding: "8px 8px 0" }}>
        {Graph
          ? <Graph graph={SOL_GRAPH} docsOn={false} focus={focus} selected={selected} onSelect={setSelected}/>
          : <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--ink-3)" }}>graph unavailable</div>}
        <div style={{ position: "absolute", left: 22, bottom: 16, display: "flex", gap: 16, alignItems: "center",
              background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 8, padding: "8px 12px" }}>
          {SOL.projects.map(p => (
            <span key={p.id} style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11.5,
                  color: focus && focus !== p.id ? "var(--ink-4)" : "var(--ink-2)" }}>
              <span className="kanji" style={{ fontSize: 14, color: p.warn ? "var(--warning)" : "var(--ink-3)" }}>{p.kanji}</span>
              <span className="mono">{p.name}</span>
            </span>
          ))}
          <span style={{ width: 1, height: 16, background: "var(--edge)" }}/>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11.5, color: "var(--ink-3)" }}>
            <svg width="22" height="6"><line x1="0" y1="3" x2="22" y2="3" stroke="var(--ink-3)" strokeWidth="1.4" strokeDasharray="5 5"/></svg>
            cross-project
          </span>
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ③ SESSIONS — every session in the solution, filterable by project
// ═══════════════════════════════════════════════════════════════════
function SolutionSessions({ filter, setFilter, forceEmpty }) {
  const rows = forceEmpty ? [] : (filter === "all" ? SOL.sessions : SOL.sessions.filter(s => s.project === filter));
  const chips = [{ id: "all", label: "All", kanji: "全" }, ...SOL.projects.map(p => ({ id: p.id, label: p.name, kanji: p.kanji }))];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--paper)" }}>
      <div style={{ borderBottom: "var(--hairline)", flexShrink: 0 }} className="px-7 pt-5 pb-4">
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span className="kanji" style={{ fontSize: 30, color: "var(--accent)", lineHeight: 1 }}>刻</span>
          <div>
            <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)" }}>Solution · Sessions</div>
            <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400, letterSpacing: "-0.01em" }}>Every session, one stream</h1>
          </div>
        </div>
        {/* filter chips */}
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 14 }}>
          {chips.map(c => {
            const on = filter === c.id;
            return (
              <button key={c.id} onClick={() => setFilter(c.id)} style={{
                display: "inline-flex", alignItems: "center", gap: 7, cursor: "pointer",
                border: on ? "1px solid var(--ink)" : "var(--hairline)", borderRadius: 999,
                padding: "6px 13px", fontSize: 12.5,
                background: on ? "var(--ink)" : "transparent",
                color: on ? "var(--paper)" : "var(--ink-2)",
              }}>
                <span className="kanji" style={{ fontSize: 14, color: on ? "var(--paper)" : "var(--ink-3)" }}>{c.kanji}</span>
                <span className={c.id === "all" ? "" : "mono"}>{c.label}</span>
              </button>
            );
          })}
        </div>
      </div>

      <div style={{ flex: 1, overflow: "auto", minHeight: 0 }}>
        {rows.length === 0 ? (
          <div style={{ height: "100%", display: "flex", flexDirection: "column", alignItems: "center",
                justifyContent: "center", gap: 12, color: "var(--ink-3)" }}>
            <span className="kanji" style={{ fontSize: 48, color: "var(--ink-4)" }}>空</span>
            <div style={{ fontSize: 15, color: "var(--ink-2)" }}>Still listening.</div>
            <div style={{ fontSize: 13, color: "var(--ink-3)" }}>No sessions in {filter === "all" ? "this solution" : projName(filter)} yet.</div>
          </div>
        ) : (
          <>
            {/* column header */}
            <div style={{ display: "grid", gridTemplateColumns: "150px 1fr 120px 90px 90px",
                  gap: 16, padding: "10px 28px", borderBottom: "var(--hairline)",
                  fontSize: 11, letterSpacing: "0.14em", textTransform: "uppercase", color: "var(--ink-3)" }}>
              <span>Project</span><span>Session</span><span>When</span><span>Length</span>
              <span style={{ textAlign: "right" }}>FTR</span>
            </div>
            {rows.map(s => (
              <div key={s.id} style={{ display: "grid", gridTemplateColumns: "150px 1fr 120px 90px 90px",
                    gap: 16, padding: "15px 28px", borderBottom: "var(--hairline)", alignItems: "center" }}>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                  <span className="kanji" style={{ fontSize: 15, color: "var(--ink-3)" }}>{projKanji(s.project)}</span>
                  <span className="mono" style={{ fontSize: 12, color: "var(--ink-2)" }}>{projName(s.project)}</span>
                </span>
                <span>
                  <span style={{ fontSize: 14, color: "var(--ink)" }}>{s.title}</span>
                  <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", marginLeft: 8 }}>{s.id}</span>
                  {s.corrections > 0 && <span className="mono" style={{ fontSize: 11, color: "var(--warning)", marginLeft: 8 }}>· {s.corrections} corrections</span>}
                </span>
                <span className="mono" style={{ fontSize: 12, color: "var(--ink-3)" }}>{s.time}</span>
                <span className="mono" style={{ fontSize: 12, color: "var(--ink-3)" }}>{s.duration}</span>
                <span style={{ textAlign: "right" }}>
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 12,
                        color: s.ftr ? "var(--success)" : "var(--warning)" }}>
                    <span style={{ width: 7, height: 7, borderRadius: "50%",
                          background: s.ftr ? "var(--success)" : "var(--warning)" }}/>
                    {s.ftr ? "first try" : "corrected"}
                  </span>
                </span>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  The window — segmented nav ties the three together
// ═══════════════════════════════════════════════════════════════════
function SolutionWindow({ initial = "dashboard", initialFilter = "all", forceEmpty = false }) {
  const [view, setView] = solS(initial);
  const [filter, setFilter] = solS(initialFilter);
  const viewLabel = (SOL_NAV.find(n => n.id === view) || {}).label.toLowerCase();
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column",
                  background: "var(--paper)", overflow: "hidden" }}>
      <SolChrome view={viewLabel}/>
      <div style={{ flex: 1, display: "grid", gridTemplateColumns: "230px 1fr", minHeight: 0 }}>
        <SolRail view={view} setView={setView} filter={filter} setFilter={setFilter}/>
        <main style={{ minHeight: 0, overflow: "hidden" }}>
          {view === "dashboard"    && <SolutionDashboard/>}
          {view === "architecture" && <SolutionArchitecture filter={filter} setFilter={setFilter}/>}
          {view === "sessions"     && <SolutionSessions filter={filter} setFilter={setFilter} forceEmpty={forceEmpty}/>}
        </main>
      </div>
    </div>
  );
}

Object.assign(window, {
  SOL, SOL_GRAPH,
  SolutionDashboard, SolutionArchitecture, SolutionSessions, SolutionWindow,
});
