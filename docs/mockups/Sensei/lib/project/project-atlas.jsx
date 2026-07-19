// Project · Atlas — a code graph view of everything inside a project.
//
// One project spans several repos, each with crates (Rust) and packages (JS),
// which contain modules, which contain classes and functions — and docs that
// reference all of it. The Atlas lets you move between those granularities and
// see how the pieces relate. Selecting a node reveals its dependencies, what
// depends on it, the docs that describe it, and whether any of them have drifted.
//
// Self-contained: authored graph data + an SVG graph renderer + an inspector,
// plus a window wrapper (ProjectAtlasWindow) for the Observatory showcase.

const { useState: atS } = React;

// ─── Granularity levels ─────────────────────────────────────
const ATLAS_LEVELS = [
  { id: "repo",    label: "Repos"    },
  { id: "package", label: "Packages" },
  { id: "module",  label: "Modules"  },
  { id: "symbol",  label: "Symbols"  },
];

// ─── The authored graph, one layout per granularity ─────────
// Positions live in a 780×520 viewBox. Edges read "from depends on to".
const ATLAS = {
  repo: {
    nodes: [
      { id: "core",  kind: "repo", kanji: "核", label: "lumen-core",  sub: "Rust · 4 crates",  x: 600, y: 170, r: 33 },
      { id: "api",   kind: "repo", kanji: "関", label: "lumen-api",   sub: "Rust · 1 crate",   x: 365, y: 230, r: 31, children: true },
      { id: "web",   kind: "repo", kanji: "面", label: "lumen-web",   sub: "TypeScript · 3 pkg", x: 150, y: 360, r: 31 },
      { id: "infra", kind: "repo", kanji: "基", label: "lumen-infra", sub: "Terraform + docs", x: 380, y: 430, r: 28 },
    ],
    edges: [
      { from: "api", to: "core" },
      { from: "web", to: "api", boundary: true },
      { from: "infra", to: "api" },
      { from: "infra", to: "web" },
    ],
    docs: [
      { id: "d-arch", label: "Architecture", attach: "core", status: "ok", x: 690, y: 90 },
    ],
  },

  // Package level shows the whole project; `focus` (a repo id) just spotlights
  // one cluster without losing the surrounding context.
  package: {
    nodes: [
      { id: "web-app",        kind: "package", repo: "web",  kanji: "頁", label: "web-app",        sub: "React app",   x: 110, y: 110, r: 27 },
      { id: "lumen-ui",       kind: "package", repo: "web",  kanji: "彩", label: "@lumen/ui",      sub: "components",  x: 85,  y: 300, r: 26 },
      { id: "lumen-client",   kind: "package", repo: "web",  kanji: "客", label: "@lumen/client",  sub: "SDK",         x: 215, y: 215, r: 26 },
      { id: "api-server",     kind: "crate",   repo: "api",  kanji: "関", label: "api-server",     sub: "axum service", x: 390, y: 235, r: 28 },
      { id: "gateway-core",   kind: "crate",   repo: "core", kanji: "門", label: "gateway-core",   sub: "routing",     x: 565, y: 105, r: 26 },
      { id: "auth",           kind: "crate",   repo: "core", kanji: "鍵", label: "auth",           sub: "5 modules",   x: 615, y: 245, r: 28, children: true },
      { id: "billing",        kind: "crate",   repo: "core", kanji: "勘", label: "billing",        sub: "3 modules",   x: 555, y: 390, r: 26 },
      { id: "ledger",         kind: "crate",   repo: "core", kanji: "簿", label: "ledger",         sub: "append-only", x: 680, y: 430, r: 25 },
    ],
    edges: [
      { from: "api-server",   to: "auth" },
      { from: "api-server",   to: "gateway-core" },
      { from: "api-server",   to: "billing" },
      { from: "gateway-core", to: "auth" },
      { from: "billing",      to: "ledger" },
      { from: "web-app",      to: "lumen-client" },
      { from: "web-app",      to: "lumen-ui" },
      { from: "lumen-client", to: "api-server", boundary: true },
    ],
    docs: [
      { id: "d-auth",    label: "Auth & Sessions", attach: "auth",       status: "drift",  x: 700, y: 170 },
      { id: "d-billing", label: "Billing model",   attach: "billing",    status: "ok",     x: 445, y: 425 },
      { id: "d-api",     label: "API reference",   attach: "api-server", status: "ok",     x: 345, y: 360 },
    ],
  },

  module: {
    // scoped to the auth crate
    auth: {
      nodes: [
        { id: "middleware", kind: "module", kanji: "層", label: "middleware", sub: "tower layer", x: 150, y: 130, r: 26 },
        { id: "session",    kind: "module", kanji: "刻", label: "session",    sub: "lifecycle",   x: 375, y: 205, r: 27, children: true },
        { id: "token",      kind: "module", kanji: "符", label: "token",      sub: "JWT + refresh", x: 600, y: 145, r: 26 },
        { id: "device",     kind: "module", kanji: "端", label: "device",     sub: "trust store", x: 195, y: 365, r: 25 },
        { id: "error",      kind: "module", kanji: "誤", label: "error",      sub: "AuthError",   x: 575, y: 380, r: 24 },
      ],
      edges: [
        { from: "session",    to: "token" },
        { from: "token",      to: "error" },
        { from: "device",     to: "session" },
        { from: "middleware", to: "session" },
        { from: "middleware", to: "token" },
      ],
      docs: [
        { id: "d-sess", label: "Session lifecycle", attach: "session", status: "drift", x: 375, y: 365 },
        { id: "d-tok",  label: "Token rotation",    attach: "token",   status: "ok",    x: 690, y: 255 },
      ],
    },
  },

  symbol: {
    // scoped to auth::session
    session: {
      nodes: [
        { id: "create_session", kind: "fn",    kanji: "生", label: "create_session", sub: "fn",    x: 165, y: 145, r: 23 },
        { id: "SessionStore",   kind: "class", kanji: "蔵", label: "SessionStore",   sub: "struct", x: 405, y: 250, r: 29 },
        { id: "refresh",        kind: "fn",    kanji: "更", label: "refresh",        sub: "fn",    x: 625, y: 135, r: 23 },
        { id: "revoke",         kind: "fn",    kanji: "消", label: "revoke",         sub: "fn",    x: 640, y: 335, r: 22 },
        { id: "validate",       kind: "fn",    kanji: "検", label: "validate",       sub: "fn",    x: 375, y: 425, r: 23 },
      ],
      edges: [
        { from: "create_session", to: "SessionStore" },
        { from: "refresh",        to: "validate" },
        { from: "refresh",        to: "SessionStore" },
        { from: "revoke",         to: "SessionStore" },
        { from: "validate",       to: "SessionStore" },
      ],
      docs: [
        { id: "d-ref", label: "refresh() contract", attach: "refresh", status: "broken", x: 690, y: 215 },
      ],
    },
  },
};

const REPO_NAMES = { core: "lumen-core", api: "lumen-api", web: "lumen-web", infra: "lumen-infra" };

// ─── Colors per node kind (token-only) ──────────────────────
function atlasFill(kind, status) {
  if (kind === "doc") return status === "broken" ? "var(--danger)" : "var(--warning)";
  if (kind === "repo")    return "var(--accent)";
  if (kind === "package") return "var(--success)";
  if (kind === "crate")   return "var(--ink)";
  if (kind === "class")   return "var(--ink)";
  if (kind === "fn")      return "var(--accent)";
  if (kind === "module")  return "var(--paper-3)";
  return "var(--ink-2)";
}
function atlasText(kind) {
  return kind === "module" ? "var(--ink)" : "var(--paper)";
}
const KIND_LABEL = {
  repo: "Repository", package: "Package", crate: "Crate",
  module: "Module", class: "Type / struct", fn: "Function", doc: "Document",
};

// ─── Geometry: trim an edge to the node rims, leave room for arrow ──
function atlasEdgePts(a, b) {
  const dx = b.x - a.x, dy = b.y - a.y;
  const d = Math.hypot(dx, dy) || 1;
  const ux = dx / d, uy = dy / d;
  return {
    x1: a.x + ux * (a.r + 3), y1: a.y + uy * (a.r + 3),
    x2: b.x - ux * (b.r + 9), y2: b.y - uy * (b.r + 9),
  };
}

// ─── The graph canvas ───────────────────────────────────────
function AtlasGraph({ graph, docsOn, focus, selected, onSelect }) {
  const byId = {};
  graph.nodes.forEach(n => { byId[n.id] = n; });
  const docNodes = docsOn ? (graph.docs || []) : [];
  docNodes.forEach(n => { byId[n.id] = { ...n, kind: "doc", r: 18 }; });

  // neighbours of the selected node (across code + doc edges)
  const neighbours = new Set();
  if (selected) {
    neighbours.add(selected);
    graph.edges.forEach(e => {
      if (e.from === selected) neighbours.add(e.to);
      if (e.to === selected) neighbours.add(e.from);
    });
    docNodes.forEach(d => { if (d.attach === selected) neighbours.add(d.id); });
  }

  function nodeOpacity(id, repo) {
    if (selected) return neighbours.has(id) ? 1 : 0.22;
    if (focus && repo && repo !== focus) return 0.34;
    return 1;
  }

  function codeEdgeStyle(e) {
    const incident = selected && (e.from === selected || e.to === selected);
    if (selected) {
      return incident
        ? { stroke: "var(--accent)", width: 2.1, op: 0.95 }
        : { stroke: "var(--ink-4)", width: 1.2, op: 0.18 };
    }
    if (focus) {
      const a = byId[e.from], b = byId[e.to];
      const inFocus = (a && a.repo === focus) && (b && b.repo === focus);
      return { stroke: "var(--ink-3)", width: 1.4, op: inFocus ? 0.6 : 0.2 };
    }
    return { stroke: "var(--ink-3)", width: 1.5, op: 0.5 };
  }

  return (
    <svg viewBox="0 0 780 520" preserveAspectRatio="xMidYMid meet"
         style={{ width: "100%", height: "100%", display: "block" }}>
      <defs>
        <marker id="atlas-arrow" markerWidth="9" markerHeight="9" refX="7" refY="4.5"
                orient="auto">
          <path d="M0,0 L9,4.5 L0,9 z" fill="context-stroke"/>
        </marker>
      </defs>

      {/* code dependency edges */}
      {graph.edges.map((e, i) => {
        const a = byId[e.from], b = byId[e.to];
        if (!a || !b) return null;
        const p = atlasEdgePts(a, b);
        const st = codeEdgeStyle(e);
        return (
          <line key={"e" + i} x1={p.x1} y1={p.y1} x2={p.x2} y2={p.y2}
                stroke={st.stroke} strokeWidth={st.width} opacity={st.op}
                strokeDasharray={e.boundary ? "5 5" : null}
                markerEnd="url(#atlas-arrow)"/>
        );
      })}

      {/* doc reference edges */}
      {docNodes.map((d, i) => {
        const a = byId[d.attach];
        if (!a) return null;
        const p = atlasEdgePts(byId[d.id], a);
        const dim = selected && !neighbours.has(d.id);
        const stroke = d.status === "broken" ? "var(--danger)" : "var(--warning)";
        return (
          <line key={"de" + i} x1={p.x1} y1={p.y1} x2={p.x2} y2={p.y2}
                stroke={stroke} strokeWidth={1.4} strokeDasharray="2 4"
                opacity={dim ? 0.15 : 0.7}/>
        );
      })}

      {/* doc nodes (small squares) */}
      {docNodes.map(d => {
        const op = nodeOpacity(d.id);
        const stroke = d.status === "broken" ? "var(--danger)" : "var(--warning)";
        const sel = selected === d.id;
        return (
          <g key={d.id} opacity={op} style={{ cursor: "pointer" }}
             onClick={() => onSelect(d.id)}>
            <rect x={d.x - 13} y={d.y - 13} width={26} height={26} rx={5}
                  fill="var(--paper)" stroke={stroke}
                  strokeWidth={sel ? 2.4 : 1.4} strokeDasharray="3 2"/>
            <text x={d.x} y={d.y + 4} textAnchor="middle"
                  style={{ fontSize: 12, fill: stroke }}>¶</text>
            <text x={d.x} y={d.y + 30} textAnchor="middle"
                  style={{ fontSize: 9.5, fill: "var(--ink-3)", fontFamily: "var(--font-mono)" }}>
              {d.label}
            </text>
          </g>
        );
      })}

      {/* code nodes */}
      {graph.nodes.map(n => {
        const op = nodeOpacity(n.id, n.repo);
        const sel = selected === n.id;
        const fill = atlasFill(n.kind);
        return (
          <g key={n.id} opacity={op} style={{ cursor: "pointer" }}
             onClick={() => onSelect(n.id)}>
            {sel && (
              <circle cx={n.x} cy={n.y} r={n.r + 6} fill="none"
                      stroke="var(--accent)" strokeWidth={2}/>
            )}
            <circle cx={n.x} cy={n.y} r={n.r} fill={fill}
                    stroke={n.kind === "module" ? "var(--ink-3)" : "none"}
                    strokeWidth={n.kind === "module" ? 1.2 : 0}/>
            <text x={n.x} y={n.y + n.r * 0.18} textAnchor="middle"
                  className="kanji"
                  style={{ fontSize: n.r * 0.7, fill: atlasText(n.kind) }}>
              {n.kanji}
            </text>
            <text x={n.x} y={n.y + n.r + 14} textAnchor="middle"
                  style={{ fontSize: 11, fill: "var(--ink)", fontFamily: "var(--font-mono)",
                           fontWeight: sel ? 600 : 400 }}>
              {n.label}
            </text>
            <text x={n.x} y={n.y + n.r + 26} textAnchor="middle"
                  style={{ fontSize: 9.5, fill: "var(--ink-3)" }}>
              {n.sub}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

// ─── Inspector ──────────────────────────────────────────────
function AtlasInspector({ graph, selectedId, docsOn, onDrill }) {
  const node = selectedId
    ? (graph.nodes.find(n => n.id === selectedId)
       || (graph.docs || []).map(d => ({ ...d, kind: "doc" })).find(d => d.id === selectedId))
    : null;

  if (!node) {
    const drift = (graph.docs || []).filter(d => d.status !== "ok").length;
    return (
      <div className="py-5 px-4" style={{ color: "var(--ink-3)" }}>
        <div style={{ fontSize: 11, letterSpacing: "0.16em", textTransform: "uppercase" }}
             className="mb-3">This view</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr auto", rowGap: 8 }}
             className="mb-4">
          <span style={{ fontSize: 12.5 }}>Nodes</span>
          <span className="mono" style={{ fontSize: 12.5, color: "var(--ink)" }}>{graph.nodes.length}</span>
          <span style={{ fontSize: 12.5 }}>Relations</span>
          <span className="mono" style={{ fontSize: 12.5, color: "var(--ink)" }}>{graph.edges.length}</span>
          <span style={{ fontSize: 12.5 }}>Docs linked</span>
          <span className="mono" style={{ fontSize: 12.5, color: "var(--ink)" }}>{(graph.docs || []).length}</span>
          {docsOn && drift > 0 && (
            <React.Fragment>
              <span style={{ fontSize: 12.5, color: "var(--warning)" }}>Doc drift</span>
              <span className="mono" style={{ fontSize: 12.5, color: "var(--warning)" }}>{drift}</span>
            </React.Fragment>
          )}
        </div>
        <p style={{ fontSize: 12.5, lineHeight: 1.6 }} className="m-0">
          Select any node to trace its dependencies, what relies on it, and the docs
          that describe it.
        </p>
      </div>
    );
  }

  const dependsOn = graph.edges.filter(e => e.from === node.id).map(e => e.to);
  const usedBy = graph.edges.filter(e => e.to === node.id).map(e => e.from);
  const docs = (graph.docs || []).filter(d => d.attach === node.id);
  const fill = atlasFill(node.kind, node.status);

  return (
    <div className="py-5 px-4">
      {/* identity */}
      <div style={{ display: "flex", alignItems: "center" }} className="gap-3 mb-1">
        <span style={{
          width: 34, height: 34, borderRadius: node.kind === "doc" ? 6 : "50%",
          background: node.kind === "doc" ? "var(--paper)" : fill,
          border: node.kind === "doc" ? `1.4px dashed ${fill}` : "none",
          display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0,
        }}>
          <span className="kanji" style={{ fontSize: 17,
                color: node.kind === "doc" ? fill : atlasText(node.kind) }}>
            {node.kind === "doc" ? "¶" : node.kanji}
          </span>
        </span>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: "0.12em", textTransform: "uppercase",
                        color: "var(--ink-3)" }}>{KIND_LABEL[node.kind]}</div>
          <div className="mono" style={{ fontSize: 14, color: "var(--ink)", fontWeight: 600 }}>
            {node.label}
          </div>
        </div>
      </div>
      <div style={{ fontSize: 12, color: "var(--ink-3)" }} className="mb-4 mt-1">{node.sub}</div>

      {node.kind === "doc" && (
        <div className="mb-4 py-2 px-3" style={{
          borderRadius: 6, fontSize: 12,
          background: node.status === "broken" ? "var(--danger-soft)" : "var(--warning-soft)",
          color: node.status === "broken" ? "var(--danger)" : "var(--warning)",
        }}>
          {node.status === "broken"
            ? "Broken reference — points at a symbol that no longer exists."
            : "Drifted — code changed after this doc was last reviewed."}
        </div>
      )}

      {dependsOn.length > 0 && (
        <AtlasRelist title={`Depends on · ${dependsOn.length}`} ids={dependsOn} graph={graph} onDrill={onDrill}/>
      )}
      {usedBy.length > 0 && (
        <AtlasRelist title={`Used by · ${usedBy.length}`} ids={usedBy} graph={graph} onDrill={onDrill}/>
      )}

      {docs.length > 0 && (
        <div className="mb-3">
          <div style={{ fontSize: 11, letterSpacing: "0.14em", textTransform: "uppercase",
                        color: "var(--ink-3)" }} className="mb-2">Linked docs</div>
          <div style={{ display: "flex", flexDirection: "column" }} className="gap-1">
            {docs.map(d => (
              <div key={d.id} style={{ display: "flex", justifyContent: "space-between",
                                       alignItems: "center", fontSize: 12 }}>
                <span style={{ color: "var(--ink-2)" }}>{d.label}</span>
                <span className="mono" style={{ fontSize: 10.5,
                      color: d.status === "ok" ? "var(--success)"
                           : d.status === "broken" ? "var(--danger)" : "var(--warning)" }}>
                  {d.status === "ok" ? "in sync" : d.status}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {node.children ? (
        <button onClick={() => onDrill(node.id)} className="zs-btn zs-btn-primary zs-btn-sm mt-2"
                style={{ width: "100%", justifyContent: "center" }}>
          Open {node.label} →
        </button>
      ) : node.kind !== "doc" ? (
        <button className="zs-btn zs-btn-secondary zs-btn-sm mt-2"
                style={{ width: "100%", justifyContent: "center" }}>
          Reveal in editor
        </button>
      ) : null}
    </div>
  );
}

function AtlasRelist({ title, ids, graph, onDrill }) {
  return (
    <div className="mb-3">
      <div style={{ fontSize: 11, letterSpacing: "0.14em", textTransform: "uppercase",
                    color: "var(--ink-3)" }} className="mb-2">{title}</div>
      <div style={{ display: "flex", flexWrap: "wrap" }} className="gap-1">
        {ids.map(id => {
          const n = graph.nodes.find(x => x.id === id);
          return (
            <span key={id} onClick={() => onDrill && n && onDrill(id, true)}
                  className="mono" style={{
                    fontSize: 11, color: "var(--ink-2)", cursor: "pointer",
                    border: "1px solid var(--edge)", borderRadius: 4, padding: "4px 8px",
                  }}>
              {n ? n.label : id}
            </span>
          );
        })}
      </div>
    </div>
  );
}

// ─── Legend ─────────────────────────────────────────────────
function AtlasLegend({ docsOn }) {
  const items = [
    { c: "var(--accent)", t: "repo / fn" },
    { c: "var(--ink)", t: "crate / type" },
    { c: "var(--success)", t: "JS package" },
    { c: "var(--paper-3)", t: "module", ring: true },
  ];
  return (
    <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }} className="gap-4">
      {items.map((it, i) => (
        <span key={i} style={{ display: "flex", alignItems: "center", fontSize: 11,
                               color: "var(--ink-3)" }} className="gap-2">
          <span style={{ width: 11, height: 11, borderRadius: "50%", background: it.c,
                         border: it.ring ? "1px solid var(--ink-3)" : "none" }}/>
          {it.t}
        </span>
      ))}
      <span style={{ display: "flex", alignItems: "center", fontSize: 11,
                     color: "var(--ink-3)" }} className="gap-2">
        <span style={{ width: 18, borderTop: "1.5px dashed var(--ink-3)" }}/>
        cross-language
      </span>
      {docsOn && (
        <span style={{ display: "flex", alignItems: "center", fontSize: 11,
                       color: "var(--warning)" }} className="gap-2">
          <span style={{ width: 11, height: 11, borderRadius: 3, background: "var(--paper)",
                         border: "1.3px dashed var(--warning)" }}/>
          doc (drift)
        </span>
      )}
    </div>
  );
}

// ─── The Atlas screen ───────────────────────────────────────
function ProjectAtlas() {
  const [level, setLevel] = atS("package");
  const [scope, setScope] = atS("all");   // package: repo focus | module/symbol: scope id
  const [selected, setSelected] = atS(null);
  const [docsOn, setDocsOn] = atS(true);
  const [crumbs, setCrumbs] = atS([{ label: "Lumen Cloud", level: "package", scope: "all" }]);

  function graphFor(lv, sc) {
    if (lv === "repo") return ATLAS.repo;
    if (lv === "package") return ATLAS.package;
    if (lv === "module") return ATLAS.module[sc] || ATLAS.module.auth;
    return ATLAS.symbol[sc] || ATLAS.symbol.session;
  }
  const graph = graphFor(level, scope);
  const focus = level === "package" && scope !== "all" ? scope : null;

  // jump to a level from the segmented control
  function goLevel(lv) {
    setSelected(null);
    if (lv === "repo") { setLevel("repo"); setScope(null); setCrumbs([{ label: "Lumen Cloud", level: "repo", scope: null }]); }
    else if (lv === "package") { setLevel("package"); setScope("all"); setCrumbs([{ label: "Lumen Cloud", level: "package", scope: "all" }]); }
    else if (lv === "module") { setLevel("module"); setScope("auth"); setCrumbs([{ label: "Lumen Cloud", level: "package", scope: "all" }, { label: "auth", level: "module", scope: "auth" }]); }
    else { setLevel("symbol"); setScope("session"); setCrumbs([{ label: "Lumen Cloud", level: "package", scope: "all" }, { label: "auth", level: "module", scope: "auth" }, { label: "session", level: "symbol", scope: "session" }]); }
  }

  // drill from a node into its children
  function drill(id) {
    const node = graph.nodes.find(n => n.id === id);
    if (!node) return;
    if (level === "repo" && id === "core") {
      setLevel("package"); setScope("core"); setSelected(null);
      setCrumbs(c => [...c, { label: "lumen-core", level: "package", scope: "core" }]);
    } else if (level === "repo") {
      // other repos: focus that cluster at package level
      setLevel("package"); setScope("all"); setSelected(null);
    } else if (level === "package" && node.children) {
      setLevel("module"); setScope("auth"); setSelected(null);
      setCrumbs(c => [...c, { label: node.label, level: "module", scope: "auth" }]);
    } else if (level === "module" && node.children) {
      setLevel("symbol"); setScope("session"); setSelected(null);
      setCrumbs(c => [...c, { label: node.label, level: "symbol", scope: "session" }]);
    }
  }

  // node clicks: select, then a second click on a drillable node drills in
  function onSelect(id) {
    const node = graph.nodes.find(n => n.id === id);
    if (selected === id && node && node.children) { drill(id); return; }
    setSelected(id);
  }

  function gotoCrumb(i) {
    const c = crumbs[i];
    setLevel(c.level); setScope(c.scope); setSelected(null);
    setCrumbs(crumbs.slice(0, i + 1));
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%",
                  background: "var(--paper)" }}>
      {/* header */}
      <div style={{ borderBottom: "var(--hairline)", flexShrink: 0 }} className="pt-5 px-6 pb-4">
        <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between" }}
             className="gap-4">
          <div style={{ display: "flex", alignItems: "center" }} className="gap-3">
            <span className="kanji" style={{ fontSize: 30, color: "var(--accent)", lineHeight: 1 }}>図</span>
            <div>
              <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase",
                            color: "var(--ink-3)" }}>Project · Atlas</div>
              <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400, letterSpacing: "-0.01em" }}>
                Code graph
              </h1>
            </div>
          </div>
          {/* docs toggle */}
          <button onClick={() => setDocsOn(v => !v)} style={{
            display: "flex", alignItems: "center", gap: 8, cursor: "pointer",
            border: "var(--hairline)", borderRadius: 6, background: docsOn ? "var(--warning-soft)" : "transparent",
            padding: "8px 12px", fontSize: 12, color: docsOn ? "var(--warning)" : "var(--ink-3)",
            whiteSpace: "nowrap",
          }}>
            <span style={{ width: 10, height: 10, borderRadius: 3,
                           background: docsOn ? "var(--warning)" : "transparent",
                           border: docsOn ? "none" : "1.3px solid var(--ink-3)" }}/>
            Docs layer
          </button>
        </div>

        {/* breadcrumb + granularity */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between",
                      flexWrap: "wrap" }} className="gap-3 mt-4">
          <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }} className="gap-1">
            {crumbs.map((c, i) => (
              <React.Fragment key={i}>
                {i > 0 && <span style={{ color: "var(--ink-4)", fontSize: 12 }}>›</span>}
                <button onClick={() => gotoCrumb(i)} className="mono" style={{
                  background: "none", border: "none", cursor: "pointer", fontSize: 12,
                  color: i === crumbs.length - 1 ? "var(--ink)" : "var(--ink-3)",
                  fontWeight: i === crumbs.length - 1 ? 600 : 400, padding: "4px 4px",
                }}>{c.label}</button>
              </React.Fragment>
            ))}
          </div>
          {/* segmented granularity */}
          <div style={{ display: "flex", background: "var(--paper-3)", borderRadius: 7, padding: 3 }}>
            {ATLAS_LEVELS.map(l => (
              <button key={l.id} onClick={() => goLevel(l.id)} style={{
                border: "none", cursor: "pointer", borderRadius: 5, padding: "8px 12px",
                fontSize: 12, fontWeight: level === l.id ? 600 : 400,
                background: level === l.id ? "var(--paper)" : "transparent",
                color: level === l.id ? "var(--ink)" : "var(--ink-3)",
                boxShadow: level === l.id ? "var(--shadow-sm)" : "none",
              }}>{l.label}</button>
            ))}
          </div>
        </div>
      </div>

      {/* body: graph + inspector */}
      <div style={{ flex: 1, display: "grid", gridTemplateColumns: "1fr 290px", minHeight: 0 }}>
        <div style={{ position: "relative", minHeight: 0, padding: "8px 8px 0" }}>
          <AtlasGraph graph={graph} docsOn={docsOn} focus={focus}
                      selected={selected} onSelect={onSelect}/>
          <div style={{ position: "absolute", left: 20, bottom: 14 }}>
            <AtlasLegend docsOn={docsOn}/>
          </div>
        </div>
        <div style={{ borderLeft: "var(--hairline)", background: "var(--paper-2)",
                      overflow: "auto", minHeight: 0 }}>
          <AtlasInspector graph={graph} selectedId={selected} docsOn={docsOn} onDrill={(id) => {
            const node = graph.nodes.find(n => n.id === id);
            if (node && node.children) drill(id); else setSelected(id);
          }}/>
        </div>
      </div>
    </div>
  );
}

// ─── Window wrapper for the Observatory showcase ────────────
function ProjectAtlasWindow() {
  const D = window.OBS_DATA;
  const project = (D && D.projects.active.find(p => p.id === "lumen-cloud")) ||
                  (D && D.projects.active[0]) || { name: "Lumen Cloud", kanji: "雲", client: "lumen-systems" };
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column",
                  background: "var(--paper)", overflow: "hidden" }}>
      <div style={{ height: 38, background: "var(--paper-2)", borderBottom: "var(--hairline)",
                    display: "flex", alignItems: "center", flexShrink: 0, position: "relative" }}>
        <div style={{ display: "flex", position: "absolute", left: 14 }} className="gap-2">
          {["var(--danger)", "var(--warning)", "var(--success)"].map(c => (
            <span key={c} style={{ width: 11, height: 11, borderRadius: "50%", background: c }}/>
          ))}
        </div>
        <div style={{ position: "absolute", left: 0, right: 0, top: 0, bottom: 0,
                      display: "flex", alignItems: "center", justifyContent: "center",
                      pointerEvents: "none" }}>
          <span style={{ fontSize: 12, color: "var(--ink-2)" }}>
            <span className="kanji" style={{ color: "var(--accent)" }}>先生</span>
            {"  ·  " + project.name}
            <span style={{ color: "var(--ink-4)" }}>{"  ·  atlas"}</span>
          </span>
        </div>
        <div style={{ position: "absolute", top: 0, left: 0, right: 0, height: 2,
                      background: "var(--accent)" }}/>
      </div>
      <div style={{ flex: 1, display: "grid", gridTemplateColumns: "230px 1fr", minHeight: 0 }}>
        {window.ProjectSidebar
          ? <window.ProjectSidebar project={project} active="atlas"/>
          : <div/>}
        <main style={{ minHeight: 0, overflow: "hidden" }}>
          <ProjectAtlas/>
        </main>
      </div>
    </div>
  );
}

Object.assign(window, { ProjectAtlas, ProjectAtlasWindow });
