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
    <div className="bg-paper-2 border-b flex items-center shrink-0 relative" style={{ height: 38 }}>
      <div style={{ left: 14 }} className="gap-2 flex absolute">
        {["var(--danger)", "var(--warning)", "var(--success)"].map(c => (
          <span className="rounded-full" key={c} style={{ width: 11, height: 11, background: c }}/>
        ))}
      </div>
      <div className="absolute flex items-center justify-center" style={{ inset: 0, pointerEvents: "none" }}>
        <span className="text-ink-2" style={{ fontSize: 12 }}>
          <span className="kanji text-accent" >先生</span>
          {"  ·  Solution · " + SOL.name}
          <span className="text-ink-4" >{"  ·  " + view}</span>
        </span>
      </div>
      <div className="absolute bg-accent" style={{ top: 0, left: 0, right: 0, height: 2 }}/>
    </div>
  );
}

// slim left rail: the engagement + its member projects
function SolRail({ view, setView, filter, setFilter }) {
  return (
    <aside className="border-r bg-paper-2 flex flex-col min-h-0" style={{ width: 230 }}>
      <div className="px-6 pt-6 pb-4 border-b" >
        <div className="gap-3 flex items-center">
          <span className="kanji text-accent" style={{ fontSize: 26, lineHeight: 1 }}>{SOL.kanji}</span>
          <div>
            <div className="font-semibold text-ink" style={{ fontSize: 15 }}>{SOL.name}</div>
            <div className="mono text-ink-3" style={{ fontSize: 11 }}>{SOL.client}</div>
          </div>
        </div>
      </div>

      <div className="px-3 pt-4 flex flex-col" style={{ gap: 2 }}>
        {SOL_NAV.map(n => (
          <button className="flex items-center w-full text-left border-0 cursor-pointer" key={n.id} onClick={() => setView(n.id)} style={{
 padding: "8px 12px", borderRadius: 6,
 fontSize: 13.5, fontWeight: view === n.id ? 600 : 400,
 background: view === n.id ? "var(--paper-3)" : "transparent",
 color: view === n.id ? "var(--ink)" : "var(--ink-2)" }}>{n.label}</button>
        ))}
      </div>

      <div className="px-6 pt-8 pb-2 uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: "0.18em" }}>Projects · 3</div>
      <div className="px-3 flex flex-col" style={{ gap: 2 }}>
        {SOL.projects.map(p => {
          const on = filter === p.id;
          return (
            <button className="flex items-center w-full text-left border-0 cursor-pointer" key={p.id} onClick={() => setFilter && setFilter(on ? "all" : p.id)} style={{ gap: 10,
 padding: "8px 12px", borderRadius: 6,
 background: on ? "var(--paper-3)" : "transparent" }}>
              <span className="kanji" style={{ fontSize: 16, color: p.warn ? "var(--warning)" : "var(--ink-3)", width: 18 }}>{p.kanji}</span>
              <span className="mono flex-1 text-ink" style={{ fontSize: 12.5 }}>{p.name}</span>
              <span className="mono" style={{ fontSize: 11, color: p.warn ? "var(--warning)" : "var(--ink-3)" }}>{Math.round(p.ftr*100)}</span>
            </button>
          );
        })}
      </div>
      <div className="flex-1" />
      <div className="px-6 py-4 border-t text-ink-4" style={{ fontSize: 11 }}>
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
    <svg className="block overflow-visible" width={w} height={h} >
      {data.map((v, i) => {
        const bh = Math.max(3, v*h), last = i === n-1;
        return <rect key={i} x={i*(barW+gap)} y={h-bh} width={barW} height={bh}
                     fill={last ? col : "var(--edge)"} opacity={last ? 1 : 0.7}/>;
      })}
    </svg>
  );
}

function SolutionDashboard({ state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="束"
    emptyTitle="No solution data yet"
    emptyHint="Add projects to this engagement and run a few sessions — the aggregate first-try-right and per-project rollup fill in here."
    errorHint="Couldn't reach the solution rollup. Check your connection and try again."
    onRetry={() => {}} />;
  const Strip = window.ObsFtrStrip;
  return (
    <div className="h-full overflow-auto bg-paper" >
      {/* aggregate header strip */}
      <div className="px-12 pt-8 pb-6 border-b">
        <div style={{ maxWidth: 1000, gap: 24 }} className="mx-auto flex items-end justify-between flex-wrap">
          <div>
            <div style={{ fontSize: 11, letterSpacing: "0.18em" }} className="mb-1 uppercase text-ink-3">Solution · aggregate</div>
            <h1 className="display m-0 font-normal" style={{ fontSize: 28, letterSpacing: "-0.01em" }}>
              Three projects, one first-try-right.
            </h1>
            <p style={{ fontSize: 14, maxWidth: 460, lineHeight: 1.6 }} className="mt-2 mb-0 text-ink-2">
              Sensei rolls the whole engagement into one signal — then shows you which project is pulling it.
            </p>
          </div>
          <div className="text-right" >
            <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: "0.18em" }}>
              First-Try-Right · 14d
            </div>
            <div className="gap-2 mt-1 flex items-baseline justify-end">
              <span className="display font-normal text-ink" style={{ fontSize: 40, lineHeight: 1 }}>
                {Math.round(SOL.ftr*100)}
              </span>
              <span className="text-ink-3" style={{ fontSize: 13 }}>%</span>
              <span className="mono ml-1" style={{ fontSize: 11, color: SOL.delta>=0 ? "var(--success)" : "var(--warning)" }}>
                {SOL.delta>=0 ? "↑" : "↓"} {Math.abs(Math.round(SOL.delta*100))}%
              </span>
            </div>
            <div className="mt-2 flex justify-end" >
              {Strip
                ? <Strip data={SOL.trend14} value={SOL.ftr} delta={SOL.delta}/>
                : <MiniStrip data={SOL.trend14}/>}
            </div>
          </div>
        </div>
      </div>

      <div style={{ maxWidth: 1000 }} className="mx-auto px-12 py-12">
        {/* per-project rollup */}
        <div style={{ fontSize: 11, letterSpacing: "0.18em" }} className="mb-4 uppercase text-ink-3">
          Per project
        </div>
        <div className="grid" style={{ gridTemplateColumns: "repeat(3, 1fr)", gap: 16 }}>
          {SOL.projects.map(p => (
            <div className="bg-paper-2 border border-paper-edge" key={p.id} style={{
 borderRadius: 10, padding: 20 }}>
              <div className="flex items-center justify-between" >
                <span className="inline-flex items-center" style={{ gap: 9 }}>
                  <span className="kanji" style={{ fontSize: 22, color: p.warn ? "var(--warning)" : "var(--ink-3)" }}>{p.kanji}</span>
                  <span className="mono text-ink" style={{ fontSize: 13 }}>{p.name}</span>
                </span>
                {p.warn && <span className="uppercase text-warning bg-warning-soft" style={{ fontSize: 10.5, letterSpacing: ".08em", padding: "4px 8px", borderRadius: 999 }}>needs eyes</span>}
              </div>
              <div style={{ gap: 4 }} className="mt-4 flex items-baseline">
                <span className="display font-normal" style={{ fontSize: 32, lineHeight: 1,
 color: p.warn ? "var(--warning)" : "var(--ink)" }}>{Math.round(p.ftr*100)}</span>
                <span className="text-ink-3" style={{ fontSize: 12 }}>% FTR</span>
              </div>
              <div className="mt-3"><MiniStrip data={p.trend} warn={p.warn}/></div>
              <div className="mono mt-3 text-ink-3" style={{ fontSize: 11 }}>{p.sessions7d} sessions · 7d</div>
            </div>
          ))}
        </div>

        {/* rollup insights */}
        <div style={{ fontSize: 11, letterSpacing: "0.18em" }} className="mt-12 mb-3 uppercase text-ink-3">
          Across the solution
        </div>
        <div className="bg-paper-2 border border-paper-edge" style={{ borderRadius: 10 }}>
          {SOL.insights.map((x, i) => (
            <div className="grid items-center" key={i} style={{ gridTemplateColumns: "auto 1fr auto",
 gap: 14, padding: "16px 16px", borderBottom: i < SOL.insights.length-1 ? "var(--hairline)" : "none" }}>
              <span className="kanji" style={{ fontSize: 22, color: toneColor(x.tone), width: 26 }}>{x.kanji}</span>
              <div>
                <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".14em" }}>
                  {x.label} <span className="text-ink-4" >· {projName(x.project)}</span>
                </div>
                <div className="text-ink" style={{ fontSize: 14, marginTop: 3 }}>{x.text}</div>
              </div>
              <span className="mono font-semibold whitespace-nowrap" style={{ fontSize: 12, color: toneColor(x.tone) }}>{x.tag}</span>
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
function SolutionArchitecture({ filter, setFilter, state = "ready" }) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="図"
    emptyTitle="No repos linked yet"
    emptyHint="Bind this solution's projects to their repos and the merged code graph appears here."
    errorHint="Couldn't build the merged graph. Try again."
    onRetry={() => {}} />;
  const [selected, setSelected] = solS(null);
  const Graph = window.AtlasGraph;
  const focus = filter === "all" ? null : filter;

  return (
    <div className="flex flex-col h-full bg-paper" >
      <div className="px-12 pt-6 pb-4 border-b shrink-0">
        <div className="flex items-start justify-between flex-wrap" style={{ gap: 16 }}>
          <div className="flex items-center" style={{ gap: 12 }}>
            <span className="kanji text-accent" style={{ fontSize: 30, lineHeight: 1 }}>図</span>
            <div>
              <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: "0.18em" }}>Solution · Architecture</div>
              <h1 className="display m-0 font-normal" style={{ fontSize: 22, letterSpacing: "-0.01em" }}>Merged code graph</h1>
            </div>
          </div>
          {/* project filter chips */}
          <div className="flex bg-paper-3" style={{ borderRadius: 7, padding: 3, gap: 2 }}>
            {[{ id: "all", label: "All projects" }, ...SOL.projects.map(p => ({ id: p.id, label: p.name }))].map(o => {
              const on = filter === o.id;
              return (
                <button className="border-0 cursor-pointer" key={o.id} onClick={() => { setFilter(o.id); setSelected(null); }} style={{ borderRadius: 5, padding: "8px 12px",
 fontSize: 12, fontWeight: on ? 600 : 400,
 background: on ? "var(--paper)" : "transparent",
 color: on ? "var(--ink)" : "var(--ink-3)",
 boxShadow: on ? "var(--shadow-sm)" : "none",
 fontFamily: o.id === "all" ? "var(--font-ui)" : "var(--font-mono)" }}>{o.label}</button>
              );
            })}
          </div>
        </div>
        <div className="text-ink-2" style={{ fontSize: 13, marginTop: 12, maxWidth: 640 }}>
          Every repo in the engagement on one canvas. <span className="text-ink-3" >Dashed edges cross a project boundary</span> — the seams sensei watches most closely.
        </div>
      </div>

      <div className="flex-1 relative min-h-0" style={{ padding: "8px 8px 0" }}>
        {Graph
          ? <Graph graph={SOL_GRAPH} docsOn={false} focus={focus} selected={selected} onSelect={setSelected}/>
          : <div className="flex items-center justify-center h-full text-ink-3" >graph unavailable</div>}
        <div className="absolute flex items-center bg-paper-2 border border-paper-edge" style={{ left: 22, bottom: 16, gap: 16, borderRadius: 8, padding: "8px 12px" }}>
          {SOL.projects.map(p => (
            <span className="inline-flex items-center" key={p.id} style={{ gap: 6, fontSize: 11.5,
 color: focus && focus !== p.id ? "var(--ink-4)" : "var(--ink-2)" }}>
              <span className="kanji" style={{ fontSize: 14, color: p.warn ? "var(--warning)" : "var(--ink-3)" }}>{p.kanji}</span>
              <span className="mono">{p.name}</span>
            </span>
          ))}
          <span style={{ width: 1, height: 16, background: "var(--edge)" }}/>
          <span className="inline-flex items-center text-ink-3" style={{ gap: 6, fontSize: 11.5 }}>
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
function SolutionSessions({ filter, setFilter, forceEmpty, state = "ready" }) {
  if (state === "loading" || state === "error") return <window.ScreenState state={state} kanji="刻"
    emptyTitle="No sessions yet"
    emptyHint="Sessions across every project in this solution land here."
    errorHint="Couldn't load the session stream. Try again."
    onRetry={() => {}} />;
  const rows = forceEmpty ? [] : (filter === "all" ? SOL.sessions : SOL.sessions.filter(s => s.project === filter));
  const chips = [{ id: "all", label: "All", kanji: "全" }, ...SOL.projects.map(p => ({ id: p.id, label: p.name, kanji: p.kanji }))];

  return (
    <div className="flex flex-col h-full bg-paper" >
      <div className="px-12 pt-6 pb-4 border-b shrink-0">
        <div className="flex items-center" style={{ gap: 12 }}>
          <span className="kanji text-accent" style={{ fontSize: 30, lineHeight: 1 }}>刻</span>
          <div>
            <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: "0.18em" }}>Solution · Sessions</div>
            <h1 className="display m-0 font-normal" style={{ fontSize: 22, letterSpacing: "-0.01em" }}>Every session, one stream</h1>
          </div>
        </div>
        {/* filter chips */}
        <div className="flex flex-wrap" style={{ gap: 8, marginTop: 14 }}>
          {chips.map(c => {
            const on = filter === c.id;
            return (
              <button className="inline-flex items-center cursor-pointer" key={c.id} onClick={() => setFilter(c.id)} style={{ gap: 7,
 border: on ? "1px solid var(--ink)" : "var(--hairline)", borderRadius: 999,
 padding: "8px 16px", fontSize: 12.5,
 background: on ? "var(--ink)" : "transparent",
 color: on ? "var(--paper)" : "var(--ink-2)" }}>
                <span className="kanji" style={{ fontSize: 14, color: on ? "var(--paper)" : "var(--ink-3)" }}>{c.kanji}</span>
                <span className={c.id === "all" ? "" : "mono"}>{c.label}</span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="flex-1 overflow-auto min-h-0" >
        {rows.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-ink-3" style={{ gap: 12 }}>
            <span className="kanji text-ink-4" style={{ fontSize: 48 }}>空</span>
            <div className="text-ink-2" style={{ fontSize: 15 }}>Still listening.</div>
            <div className="text-ink-3" style={{ fontSize: 13 }}>No sessions in {filter === "all" ? "this solution" : projName(filter)} yet.</div>
          </div>
        ) : (
          <>
            {/* column header */}
            <div className="grid border-b uppercase text-ink-3" style={{ gridTemplateColumns: "150px 1fr 120px 90px 90px",
 gap: 16, padding: "12px 24px",
 fontSize: 11, letterSpacing: "0.14em" }}>
              <span>Project</span><span>Session</span><span>When</span><span>Length</span>
              <span className="text-right" >FTR</span>
            </div>
            {rows.map(s => (
              <div className="grid border-b items-center" key={s.id} style={{ gridTemplateColumns: "150px 1fr 120px 90px 90px",
 gap: 16, padding: "16px 24px" }}>
                <span className="inline-flex items-center" style={{ gap: 8 }}>
                  <span className="kanji text-ink-3" style={{ fontSize: 15 }}>{projKanji(s.project)}</span>
                  <span className="mono text-ink-2" style={{ fontSize: 12 }}>{projName(s.project)}</span>
                </span>
                <span>
                  <span className="text-ink" style={{ fontSize: 14 }}>{s.title}</span>
                  <span className="mono text-ink-4" style={{ fontSize: 11, marginLeft: 8 }}>{s.id}</span>
                  {s.corrections > 0 && <span className="mono text-warning" style={{ fontSize: 11, marginLeft: 8 }}>· {s.corrections} corrections</span>}
                </span>
                <span className="mono text-ink-3" style={{ fontSize: 12 }}>{s.time}</span>
                <span className="mono text-ink-3" style={{ fontSize: 12 }}>{s.duration}</span>
                <span className="text-right" >
                  <span className="inline-flex items-center" style={{ gap: 6, fontSize: 12,
 color: s.ftr ? "var(--success)" : "var(--warning)" }}>
                    <span className="rounded-full" style={{ width: 7, height: 7,
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
function SolutionWindow({ initial = "dashboard", initialFilter = "all", forceEmpty = false, state = "ready" }) {
  const [view, setView] = solS(initial);
  const [filter, setFilter] = solS(initialFilter);
  const viewLabel = (SOL_NAV.find(n => n.id === view) || {}).label.toLowerCase();
  return (
    <div className="w-full h-full flex flex-col bg-paper overflow-hidden" >
      <SolChrome view={viewLabel}/>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: "230px 1fr" }}>
        <SolRail view={view} setView={setView} filter={filter} setFilter={setFilter}/>
        <main className="min-h-0 overflow-hidden" >
          {view === "dashboard"    && <SolutionDashboard state={state}/>}
          {view === "architecture" && <SolutionArchitecture filter={filter} setFilter={setFilter} state={state}/>}
          {view === "sessions"     && <SolutionSessions filter={filter} setFilter={setFilter} forceEmpty={forceEmpty} state={state}/>}
        </main>
      </div>
    </div>
  );
}

Object.assign(window, {
  SOL, SOL_GRAPH,
  SolutionDashboard, SolutionArchitecture, SolutionSessions, SolutionWindow,
});
