// Shared pieces used by all three Project-page variations.
// Exposes: ProjHeader, ProjSettings, ProjGraphLens, ProjPatterns, ProjRecommendations,
//   ProjOverview, ProjSessions, ProjFiles, ProjActionDrawer

const { useState: pS, useMemo: pM } = React;

// ───────────────────────────────────────────────────────────
// Tiny helpers

function ProjMiniSpark({ data, w = 110, h = 28, color = 'var(--accent)' }) {
  const min = Math.min(...data), max = Math.max(...data);
  const range = max - min || 1;
  const step = w / (data.length - 1);
  const pts = data.map((v, i) => [i * step, h - (h - 2) * ((v - min) / range) - 1]);
  const d = pts.map((p, i) => (i ? "L" : "M") + p[0].toFixed(1) + " " + p[1].toFixed(1)).join(" ");
  return (
    <svg className="block" width={w} height={h} style={{ color }}>
      <path d={d} className="sparkline-path"/>
      <circle cx={pts[pts.length-1][0]} cy={pts[pts.length-1][1]} r={2} fill="currentColor"/>
    </svg>
  );
}

// ───────────────────────────────────────────────────────────
// Project header — used at the top of every layout
function ProjHeader({ project, onBack, showBack = true }) {
  return (
    <div className="gap-6 pt-6 pb-4 px-12 border-b flex items-start bg-paper" >
      {showBack && (
        <button onClick={onBack}
 style={{
 fontSize: 11, border: 'var(--ink-line)', borderRadius: 5
 }} className="py-1 px-2 text-ink-3" >
          ← all projects
        </button>
      )}
      <div className="kanji text-accent" style={{ fontSize: 56, lineHeight: 1, marginTop: -4 }}>
        {project.kanji}
      </div>
      <div className="flex-1 min-w-0" >
        <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
          Project · {project.client}
        </div>
        <h1 className="display mt-0 mb-1 font-normal" style={{
 fontSize: 28,
 letterSpacing: '-0.01em'
 }}>
          {project.name}
        </h1>
        <p style={{
 fontSize: 13, maxWidth: 560, lineHeight: 1.5
 }} className="m-0 text-ink-2 italic" >
          "{project.goal}"
        </p>
      </div>
      <div className="gap-6 py-1 px-0 flex items-center text-ink-2" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >FTR · 14d</div>
          <div className="gap-1 flex items-baseline" >
            <span className="display font-normal" style={{ fontSize: 22 }}>
              {Math.round(project.ftr * 100)}
            </span>
            <span className="mono" style={{ fontSize: 11,
                          color: project.ftr >= project.ftrPrev ? 'var(--success)' : 'var(--warning)' }}>
              {project.ftr >= project.ftrPrev ? "↑" : "↓"}
              {Math.abs(Math.round((project.ftr - project.ftrPrev) * 100))}
            </span>
          </div>
        </div>
        <ProjMiniSpark data={project.ftr14 || window.SENSEI_DATA.ftrHistory}/>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >Sessions · 7d</div>
          <div className="display font-normal" style={{ fontSize: 22 }}>{project.sessions7d}</div>
        </div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >Preferred ACP</div>
          <div className="mono" style={{ fontSize: 13 }}>{project.preferredAcp}</div>
        </div>
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
// Overview: a brief, calm summary pane
function ProjOverview({ project, openAction }) {
  const D = window.PROJECT_DATA;
  const recs = D.recommendations;
  return (
    <div style={{ gridTemplateColumns: '1.35fr 1fr' }} className="gap-8 py-6 px-12 grid" >
      <div>
        <SectionHeading k="紋" label="Repos in this project"/>
        <div className="flex flex-col" >
          {project.repos.map(r => (
            <div key={r.id} style={{ gridTemplateColumns: '1fr auto auto' }} className="gap-3 py-3 px-1 grid items-baseline border-b" >
              <div>
                <div className="text-ink" style={{ fontSize: 13 }}>{r.id}</div>
                <div className="mono text-ink-4" style={{ fontSize: 11 }}>{r.path}</div>
              </div>
              <div className="mono text-ink-3" style={{ fontSize: 11 }}>{r.lang}</div>
              <div className="mono text-ink-3" style={{ fontSize: 11 }}>{r.size}</div>
            </div>
          ))}
        </div>

        <div style={{ height: 32 }}/>
        <SectionHeading k="師" label="What sensei recommends"
                        right={<span className="mono text-ink-3" style={{ fontSize: 11 }}>
                          {recs.length} open
                        </span>}/>
        <div className="gap-2 flex flex-col" >
          {recs.map(r => <ProjRecCard key={r.id} rec={r} openAction={openAction}/>)}
        </div>
      </div>

      <div>
        <SectionHeading k="急" label="Hotspots"/>
        <div className="gap-1 mb-6 flex flex-col" >
          {D.files.filter(f => f.tags.includes("hot") || f.tags.includes("god-node")).slice(0, 5).map(f => (
            <div key={f.path} style={{ gridTemplateColumns: '1fr auto' }} className="gap-2 py-2 px-1 grid items-baseline border-b" >
              <div>
                <div className="mono text-ink" style={{ fontSize: 11 }}>{f.path}</div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
                  {f.repo} · rework {f.rework}× {f.tags.length ? "· " + f.tags.join(", ") : ""}
                </div>
              </div>
              <span className="mono text-warning" style={{ fontSize: 11 }}>
                {f.rework}×
              </span>
            </div>
          ))}
        </div>

        <SectionHeading k="紋" label="Patterns in use"/>
        <div className="gap-1 flex flex-col" >
          {D.patterns.followed.slice(0, 4).map(p => (
            <div key={p.id} className="py-2 px-1 border-b" >
              <div className="flex items-baseline justify-between" >
                <div className="text-ink" style={{ fontSize: 13 }}>{p.name}</div>
                <span className="mono" style={{ fontSize: 11,
                      color: p.status === "rule" ? 'var(--success)' :
                             p.status === "gap" ? 'var(--warning)' : 'var(--ink-3)' }}>
                  {p.status}
                </span>
              </div>
              <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
                {p.family} · {p.places} places
              </div>
            </div>
          ))}
        </div>
        {D.patterns.antiPatterns.length > 0 && (
          <div style={{ borderLeft: '2px solid var(--warning)',
 borderRadius: 5, fontSize: 11,
 lineHeight: 1.5
 }} className="mt-3 py-2 px-3 bg-warning-soft text-ink-2" >
            <span className="kanji mr-1 text-warning" style={{
 fontSize: 13 }}>避</span>
            {D.patterns.antiPatterns.length} anti-patterns detected —{" "}
            {D.patterns.antiPatterns.filter(a => a.suggest).length} have suggested fixes.
          </div>
        )}
      </div>
    </div>
  );
}

function SectionHeading({ k, label, right }) {
  return (
    <div className="mb-3 flex items-baseline justify-between" >
      <div className="gap-2 flex items-baseline" >
        <span className="kanji text-accent" style={{ fontSize: 15 }}>{k}</span>
        <h2 className="display m-0 font-normal" style={{ fontSize: 15 }}>{label}</h2>
      </div>
      {right}
    </div>
  );
}

function ProjRecCard({ rec, openAction }) {
  const tone =
    rec.urgency === "high" ? 'var(--accent)' :
    rec.urgency === "medium" ? 'var(--warning)' : 'var(--ink-3)';
  return (
    <div style={{
 borderRadius: 6,
 borderLeft: `2px solid ${tone}`,
 gridTemplateColumns: 'auto 1fr auto' }} className="gap-3 py-3 px-4 bg-paper-2 border border-paper-edge grid items-start" >
      <span className="kanji" style={{ fontSize: 22, color: tone, lineHeight: 1 }}>{rec.kanji}</span>
      <div>
        <div style={{ fontSize: 13, lineHeight: 1.45 }} className="mb-1 text-ink" >
          {rec.title}
        </div>
        <div className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55 }}>
          {rec.why}
        </div>
        <div style={{
 fontSize: 11 }} className="mono gap-2 mt-2 flex items-center text-ink-3">
          <span className="text-accent" >· {rec.impact}</span>
          <span>· {rec.evidence.join(" · ")}</span>
        </div>
      </div>
      <div className="gap-1 flex flex-col" >
        <button onClick={() => openAction(rec, "send")}
 style={{
 fontSize: 11,
 borderRadius: 5 }} className="py-2 px-3 bg-ink text-paper whitespace-nowrap" >
          send to {rec.defaultAcp} →
        </button>
        <button onClick={() => openAction(rec, "palette")}
 style={{
 fontSize: 11, border: 'var(--ink-line)',
 borderRadius: 5 }} className="py-1 px-3 text-ink-2 whitespace-nowrap" >
          customize prompt
        </button>
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
// Graph lens — three modes: graph · matrix · hairball
function ProjGraphLens({ project }) {
  const [lens, setLens] = pS("graph"); // graph | matrix | hairball
  const [overlay, setOverlay] = pS("rework");
  const D = window.PROJECT_DATA.graph;

  return (
    <div className="py-6 px-12" >
      <div className="mb-4 flex items-center justify-between" >
        <div className="gap-3 flex items-baseline" >
          <span className="kanji text-accent" style={{ fontSize: 17 }}>構</span>
          <h2 className="display m-0 font-normal" style={{ fontSize: 17 }}>
            Code visualization
          </h2>
          <span className="text-ink-3" style={{ fontSize: 11 }}>
            — three lenses on the same graph
          </span>
        </div>
        <div style={{ borderRadius: 6 }} className="gap-1 p-1 flex bg-paper-3" >
          {[["graph","Call graph"],["matrix","Matrix"],["hairball","Clusters"]].map(([id, lbl]) => (
            <button key={id} onClick={() => setLens(id)}
                    style={{
 fontSize: 11,
                             borderRadius: 4,
                             background: lens === id ? 'var(--paper)' : 'transparent',
                             color: lens === id ? 'var(--ink)' : 'var(--ink-3)'
}} className="py-1 px-3" >
              {lbl}
            </button>
          ))}
        </div>
      </div>

      {/* Overlay chips */}
      <div className="gap-2 mb-3 flex flex-wrap" >
        {[
          ["rework",    "繰", "Rework heat"],
          ["duplicates","双", "Duplicate clusters"],
          ["patterns",  "紋", "Patterns"],
          ["hotspots",  "急", "God-nodes / hotspots"],
          ["stale",     "旧", "Stale / drift"]
        ].map(([id, k, lbl]) => {
          const on = overlay === id;
          return (
            <button key={id} onClick={() => setOverlay(id)}
 style={{ fontSize: 11,
 borderRadius: 999,
 background: on ? 'var(--accent-soft)' : 'var(--paper-2)',
 border: on ? '1px solid transparent' : 'var(--hairline)',
 color: on ? 'var(--accent)' : 'var(--ink-2)'
 }} className="gap-2 py-1 px-3 inline-flex items-center" >
              <span className="kanji" style={{ fontSize: 11 }}>{k}</span>
              {lbl}
            </button>
          );
        })}
      </div>

      <div className="relative bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 10 }}>
        {lens === "graph"    && <LensGraph    D={D} overlay={overlay}/>}
        {lens === "matrix"   && <LensMatrix   D={D} overlay={overlay}/>}
        {lens === "hairball" && <LensHairball D={D} overlay={overlay}/>}
      </div>

      {/* Legend / selected detail */}
      {overlay === "duplicates" && D.duplicates.length > 0 && (
        <div style={{
 borderLeft: '2px solid var(--warning)', borderRadius: 6,
 fontSize: 13 }} className="mt-3 py-3 px-3 bg-paper-2 border border-paper-edge text-ink-2" >
          <div className="gap-2 flex items-baseline" >
            <span className="kanji text-warning" style={{ fontSize: 15 }}>双</span>
            <b className="text-ink font-medium" >{D.duplicates[0].title}</b>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              confidence {Math.round(D.duplicates[0].confidence * 100)}%
            </span>
          </div>
          <div className="mt-1 text-ink-3" >
            {D.duplicates[0].sketch} · in <span className="mono">{D.duplicates[0].files.join(" · ")}</span>
          </div>
        </div>
      )}
    </div>
  );
}

// Graph lens — node-link force-layout style (positions pre-computed in data)
function LensGraph({ D, overlay }) {
  const W = 820, H = 420;
  const nodeColor = (n) => {
    if (overlay === "rework"    && n.rework >= 5) return 'var(--warning)';
    if (overlay === "hotspots"  && n.hot)         return 'var(--accent)';
    if (overlay === "stale"     && n.stale >= 10) return 'var(--ink-4)';
    if (overlay === "duplicates"&& n.dup)         return 'var(--warning)';
    if (overlay === "patterns")                    return 'var(--success)';
    return 'var(--ink-3)';
  };
  const nodeSize = (n) => {
    if (overlay === "rework")   return 6 + n.rework * 2;
    if (overlay === "hotspots") return 6 + Math.min(18, n.fan * 0.45);
    if (overlay === "stale")    return 6 + Math.min(12, n.stale * 0.5);
    return 6 + n.size * 3;
  };
  const pos = (n) => [n.x * W, n.y * H];

  return (
    <div className="p-4" >
      <svg className="block" width={W} height={H} >
        {/* edges */}
        {D.edges.map(([a, b], i) => {
          const na = D.nodes.find(n => n.id === a);
          const nb = D.nodes.find(n => n.id === b);
          if (!na || !nb) return null;
          const [x1, y1] = pos(na), [x2, y2] = pos(nb);
          return <line key={i} x1={x1} y1={y1} x2={x2} y2={y2}
                       stroke="var(--ink)" strokeOpacity="0.12" strokeWidth="1"/>;
        })}
        {/* nodes */}
        {D.nodes.map(n => {
          const [x, y] = pos(n);
          return (
            <g key={n.id}>
              <circle cx={x} cy={y} r={nodeSize(n)} fill={nodeColor(n)} opacity={0.9}/>
              <text x={x + nodeSize(n) + 6} y={y + 3} fontFamily="var(--font-mono)"
                    fontSize="10" fill="var(--ink-2)">
                {n.id}
              </text>
            </g>
          );
        })}
      </svg>
      <div style={{ fontSize: 11 }} className="mono mt-1 flex justify-between text-ink-4">
        <span>{D.nodes.length} files · {D.edges.length} edges</span>
        <span>overlay · {overlay}</span>
      </div>
    </div>
  );
}

// Matrix lens — rows of files, cells colored by overlay
function LensMatrix({ D, overlay }) {
  const sorted = [...D.nodes].sort((a, b) => a.repo.localeCompare(b.repo) || a.id.localeCompare(b.id));
  const metric = (n) => overlay === "rework" ? n.rework : overlay === "stale" ? n.stale :
                        overlay === "hotspots" ? n.fan : overlay === "duplicates" ? (n.dup ? 1 : 0) :
                        n.size;
  const max = Math.max(...sorted.map(metric), 1);
  const colorFor = (v) => {
    const t = v / max;
    const base = overlay === "rework"     ? '72 0.12 75' :
                 overlay === "hotspots"   ? '58 0.15 35' :
                 overlay === "stale"      ? '50 0.01 50' :
                 overlay === "duplicates" ? '72 0.12 75' : '62 0.08 160';
    return `oklch(${base} / ${0.1 + t * 0.7})`;
  };
  return (
    <div style={{
 gridTemplateColumns: 'repeat(6, 1fr)'
 }} className="py-4 px-6 gap-2 grid" >
      {sorted.map(n => {
        const v = metric(n);
        return (
          <div key={n.id} style={{
 borderRadius: 5,
 background: colorFor(v),
 minHeight: 76 }} className="py-3 px-2 border border-paper-edge flex flex-col justify-between" >
            <div>
              <div className="mono text-ink" style={{ fontSize: 11,
 wordBreak: 'break-all', lineHeight: 1.3 }}>
                {n.id}
              </div>
              <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                {n.repo}
              </div>
            </div>
            <div className="mono font-medium text-ink text-right" style={{ fontSize: 13 }}>
              {v}
            </div>
          </div>
        );
      })}
    </div>
  );
}

// Hairball clusters — group bubbles with inner nodes
function LensHairball({ D, overlay }) {
  const groups = pM(() => {
    const g = {};
    D.nodes.forEach(n => {
      const key = n.repo;
      if (!g[key]) g[key] = { id: key, nodes: [] };
      g[key].nodes.push(n);
    });
    return Object.values(g);
  }, [D]);
  const W = 820, H = 420;
  const groupPos = [[0.25, 0.5], [0.55, 0.5], [0.82, 0.5]];

  return (
    <div className="p-4" >
      <svg width={W} height={H}>
        {groups.map((g, gi) => {
          const [cx, cy] = [groupPos[gi][0] * W, groupPos[gi][1] * H];
          const r = 120;
          return (
            <g key={g.id}>
              <circle cx={cx} cy={cy} r={r} fill="var(--paper-3)" opacity={0.5}/>
              <text x={cx} y={cy - r - 6} textAnchor="middle"
                    fontFamily="var(--font-display)" fontSize="13" fill="var(--ink-2)">
                {g.id}
              </text>
              {g.nodes.map((n, ni) => {
                const ang = (ni / g.nodes.length) * Math.PI * 2;
                const nx = cx + Math.cos(ang) * (r * 0.6);
                const ny = cy + Math.sin(ang) * (r * 0.6);
                const color =
                  overlay === "rework"     && n.rework >= 5 ? 'var(--warning)' :
                  overlay === "hotspots"   && n.hot         ? 'var(--accent)' :
                  overlay === "duplicates" && n.dup         ? 'var(--warning)' :
                  overlay === "stale"      && n.stale >= 10 ? 'var(--ink-4)' :
                  overlay === "patterns"                    ? 'var(--success)' :
                  'var(--ink-3)';
                return (
                  <g key={n.id}>
                    <circle cx={nx} cy={ny} r={4 + n.size * 2.2} fill={color} opacity={0.85}/>
                    <text x={nx} y={ny + 18} textAnchor="middle"
                          fontFamily="var(--font-mono)" fontSize="9" fill="var(--ink-3)">
                      {n.id.split('/').pop()}
                    </text>
                  </g>
                );
              })}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
// Patterns tab — two sides: patterns to follow, anti-patterns to avoid.
// Anti-patterns that have a `suggest` link cross-reference the
// constructive pattern that would fix them.
function ProjPatterns({ openAction }) {
  const P = window.PROJECT_DATA.patterns;
  const [side, setSide] = pS("follow");   // "follow" | "avoid"
  const list = side === "follow" ? P.followed : P.antiPatterns;
  const [focusId, setFocusId] = pS(list[0].id);
  // reset focus when side flips
  React.useEffect(() => { setFocusId(list[0].id); }, [side]);
  const focus = list.find(x => x.id === focusId) || list[0];

  return (
    <div style={{ padding: '32px 32px 48px' }}>
      {/* Section header — matches the other project panes */}
      <div className="gap-4 mb-6 flex items-end" >
        <span className="kanji text-accent" style={{ fontSize: 56, lineHeight: 1 }}>紋</span>
        <div className="flex-1" >
          <div style={{ fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            This project · patterns sensei sees
          </div>
          <h1 className="display m-0 font-normal" style={{ fontSize: 28, letterSpacing: '-0.01em' }}>
            Patterns
          </h1>
        </div>
      </div>

      {/* Toggle */}
      <div className="gap-3 mb-4 flex items-center" >
        <div style={{ borderRadius: 6
 }} className="p-1 flex bg-paper-2 border border-paper-edge" >
          <PatSideBtn on={side === "follow"} onClick={() => setSide("follow")}
                      kanji="紋" label="Patterns in use" count={P.followed.length}/>
          <PatSideBtn on={side === "avoid"} onClick={() => setSide("avoid")}
                      kanji="避" label="Anti-patterns" count={P.antiPatterns.length}
                      warn/>
        </div>
        <span className="text-ink-3" style={{ fontSize: 11, lineHeight: 1.5, maxWidth: 440 }}>
          {side === "follow"
            ? "Constructive patterns sensei detects across your code — promoted to rules once adopted."
            : "Duplication, god-nodes, monoliths. Where fixable, sensei suggests a pattern that would resolve it."}
        </span>
      </div>

      <div style={{ gridTemplateColumns: '1fr 1.1fr' }} className="gap-8 grid" >
        {/* LEFT — list */}
        <div className="flex flex-col" >
          {side === "follow"
            ? P.followed.map(p => (
                <FollowRow key={p.id} p={p}
                           on={focusId === p.id} onClick={() => setFocusId(p.id)}/>
              ))
            : P.antiPatterns.map(a => (
                <AntiRow key={a.id} a={a}
                         on={focusId === a.id} onClick={() => setFocusId(a.id)}/>
              ))}
        </div>

        {/* RIGHT — detail */}
        <div>
          {side === "follow"
            ? <FollowDetail p={focus} openAction={openAction}/>
            : <AntiDetail a={focus} allFollowed={P.followed} openAction={openAction}
                          jumpToFollowed={(id) => { setSide("follow"); setFocusId(id); }}/>}
        </div>
      </div>
    </div>
  );
}

function PatSideBtn({ on, onClick, kanji, label, count, warn }) {
  return (
    <button onClick={onClick}
 style={{
 fontSize: 13, borderRadius: 4,
 background: on ? 'var(--ink)' : 'transparent',
 color: on ? 'var(--paper)' : 'var(--ink-2)'
 }} className="py-2 px-3 gap-2 inline-flex items-center" >
      <span className="kanji" style={{ fontSize: 13,
                    color: on ? 'var(--paper)' : (warn ? 'var(--warning)' : 'var(--accent)') }}>
        {kanji}
      </span>
      {label}
      <span className="mono" style={{ fontSize: 11,
                    color: on ? 'var(--paper)' : 'var(--ink-4)', opacity: 0.85 }}>
        {count}
      </span>
    </button>
  );
}

function FollowRow({ p, on, onClick }) {
  const tone =
    p.status === "rule"      ? 'var(--success)' :
    p.status === "gap"       ? 'var(--warning)' :
    p.status === "suggested" ? 'var(--accent)'  : 'var(--ink-3)';
  const bg =
    p.status === "rule"      ? 'var(--success-soft)' :
    p.status === "gap"       ? 'var(--warning-soft)' :
    p.status === "suggested" ? 'var(--accent-soft)'  : 'var(--paper-3)';
  return (
    <button onClick={onClick}
 style={{ gridTemplateColumns: 'auto 1fr auto',
 background: on ? 'var(--paper-2)' : 'transparent'
 }} className="gap-3 py-3 px-3 grid items-start text-left border-b" >
      <span className="kanji" style={{ fontSize: 17, color: tone }}>{p.kanji}</span>
      <div>
        <div className="text-ink" style={{ fontSize: 13 }}>{p.name}</div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
          {p.family} · {p.places} places · {p.recent}
        </div>
      </div>
      <span className="mono py-1 px-2" style={{
 fontSize: 11, color: tone,
                    background: bg, borderRadius: 3
}}>
        {p.status}
      </span>
    </button>
  );
}

function AntiRow({ a, on, onClick }) {
  const sevTone =
    a.severity === "high"   ? 'var(--accent)' :
    a.severity === "medium" ? 'var(--warning)' : 'var(--ink-3)';
  const sevBg =
    a.severity === "high"   ? 'var(--accent-soft)' :
    a.severity === "medium" ? 'var(--warning-soft)' : 'var(--paper-3)';
  return (
    <button onClick={onClick}
 style={{ gridTemplateColumns: 'auto 1fr auto',
 background: on ? 'var(--paper-2)' : 'transparent'
 }} className="gap-3 py-3 px-3 grid items-start text-left border-b" >
      <span className="kanji" style={{ fontSize: 17, color: sevTone }}>{a.kanji}</span>
      <div>
        <div className="text-ink" style={{ fontSize: 13 }}>{a.name}</div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
          {a.type} · {a.occurrences}× · {a.suggest ? `fix: ${a.suggest.name}` : "no pattern suggested"}
        </div>
      </div>
      <span className="mono py-1 px-2" style={{
 fontSize: 11, color: sevTone,
                    background: sevBg, borderRadius: 3
}}>
        {a.severity}
      </span>
    </button>
  );
}

function FollowDetail({ p, openAction }) {
  return (
    <>
      <SectionHeading k={p.kanji} label={p.name}/>
      <div style={{
 fontSize: 11,
 letterSpacing: '0.06em' }} className="mb-2 text-ink-3 uppercase" >
        {p.family}
      </div>
      <div style={{ fontSize: 13, lineHeight: 1.55 }} className="mb-3 text-ink-2" >
        {p.summary}
      </div>
      <div style={{ fontSize: 13, lineHeight: 1.6 }} className="mb-3 text-ink-3" >
        Detected in <span className="mono text-ink" >{p.places}</span> places.
        Confidence <span className="mono text-ink" >{Math.round(p.confidence * 100)}%</span>.
        First seen in <span className="mono">{p.file}</span>.
      </div>
      <div style={{ borderRadius: 6,
 fontFamily: 'var(--font-mono)', fontSize: 13, whiteSpace: 'pre-wrap'
 }} className="py-3 px-4 mb-4 bg-paper-2 border border-paper-edge text-ink" >
        {p.example}
      </div>
      {p.enforcement && (
        <div style={{ borderRadius: 6,
 borderLeft: '2px solid var(--success)', fontSize: 13 }} className="py-2 px-3 mb-3 bg-success-soft text-ink-2" >
          {p.enforcement}
        </div>
      )}
      {p.status === "gap" && (
        <div style={{ borderRadius: 6,
 borderLeft: '2px solid var(--warning)' }} className="p-3 gap-3 bg-warning-soft flex items-center" >
          <div className="flex-1 text-ink-2" style={{ fontSize: 13 }}>
            This pattern is recommended but missing. Adopt it as a project rule?
          </div>
          <button onClick={() => openAction({
 id: "ad-hoc",
 defaultAcp: "claude-code",
 promptTitle: `Adopt pattern: ${p.name}`,
 prompt: `Adopt pattern "${p.name}" as a project rule.\n\n${p.summary}\n\nGenerate .sensei/rules/${p.id}.md.`
 }, "palette")}
 style={{
 fontSize: 11,
 borderRadius: 5 }} className="py-2 px-3 bg-ink text-paper whitespace-nowrap" >
            Adopt →
          </button>
        </div>
      )}
      {p.status === "suggested" && (
        <div style={{ borderRadius: 6,
 borderLeft: '2px solid var(--accent)' }} className="p-3 gap-3 bg-accent-soft flex items-center" >
          <div className="flex-1 text-ink-2" style={{ fontSize: 13 }}>
            Emerging pattern — appears in {p.places} places but not yet a project rule. Promote?
          </div>
          <button onClick={() => openAction({
 id: "ad-hoc",
 defaultAcp: "claude-code",
 promptTitle: `Promote pattern: ${p.name}`,
 prompt: `Promote pattern "${p.name}" to a project rule.\n\n${p.summary}\n\nExample:\n${p.example}`
 }, "palette")}
 style={{
 fontSize: 11,
 borderRadius: 5 }} className="py-2 px-3 bg-ink text-paper whitespace-nowrap" >
            Promote →
          </button>
        </div>
      )}
    </>
  );
}

function AntiDetail({ a, allFollowed, jumpToFollowed, openAction }) {
  const sevTone =
    a.severity === "high"   ? 'var(--accent)' :
    a.severity === "medium" ? 'var(--warning)' : 'var(--ink-3)';
  return (
    <>
      <SectionHeading k={a.kanji} label={a.name}/>
      <div className="gap-2 mb-3 flex" >
        <span className="mono py-1 px-2" style={{
 fontSize: 11, color: sevTone, borderRadius: 3,
                      background: a.severity === "high" ? 'var(--accent-soft)' :
                                  a.severity === "medium" ? 'var(--warning-soft)' : 'var(--paper-3)'
}}>
          {a.severity} · {a.type}
        </span>
        <span className="mono py-1 px-2 text-ink-3 bg-paper-3" style={{
 fontSize: 11, borderRadius: 3 }}>
          {a.occurrences}× occurrences
        </span>
      </div>
      <div style={{ fontSize: 13, lineHeight: 1.55 }} className="mb-3 text-ink-2" >
        {a.summary}
      </div>

      {/* Occurrence list */}
      <div className="mb-4" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
          Where
        </div>
        <div style={{ borderRadius: 6
 }} className="gap-1 py-2 px-2 flex flex-col bg-paper-2 border border-paper-edge" >
          {a.places.map((p, i) => (
            <div key={i} className="mono py-1 px-0 text-ink-2" style={{
 fontSize: 11 }}>
              · {p}
            </div>
          ))}
        </div>
      </div>

      <div style={{ borderRadius: 6,
 fontFamily: 'var(--font-mono)', fontSize: 13, whiteSpace: 'pre-wrap'
 }} className="py-3 px-4 mb-4 bg-paper-2 border border-paper-edge text-ink" >
        {a.example}
      </div>

      {/* Suggested fix cross-link */}
      {a.suggest && (
        <div style={{ borderRadius: 6,
 borderLeft: '2px solid var(--success)'
 }} className="p-3 bg-success-soft" >
          <div className="gap-2 mb-1 flex items-baseline" >
            <span className="kanji text-success" style={{ fontSize: 13 }}>紋</span>
            <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>Suggested pattern</span>
            <span className="text-ink" style={{ fontSize: 13 }}>{a.suggest.name}</span>
          </div>
          <div style={{
 fontSize: 13, lineHeight: 1.55
 }} className="mb-2 text-ink-2" >
            {a.suggest.reason}
          </div>
          <div className="gap-2 flex" >
            {allFollowed.find(f => f.id === a.suggest.patternId) && (
              <button onClick={() => jumpToFollowed(a.suggest.patternId)}
 style={{
 fontSize: 11,
 borderRadius: 4 }} className="py-1 px-3 bg-paper text-ink-2 border border-paper-edge" >
                See {a.suggest.name} →
              </button>
            )}
            <button onClick={() => openAction({
 id: "ad-hoc",
 defaultAcp: "claude-code",
 promptTitle: `Refactor: ${a.name}`,
 prompt: `Refactor "${a.name}" using the ${a.suggest.name} pattern.\n\n${a.suggest.reason}\n\nSites:\n${a.places.map(x => " - " + x).join("\n")}`
 }, "palette")}
 style={{
 fontSize: 11,
 borderRadius: 4
 }} className="py-1 px-3 bg-ink text-paper" >
              Refactor with this pattern →
            </button>
          </div>
        </div>
      )}

      {!a.suggest && (
        <div style={{ borderRadius: 6,
 borderLeft: '2px solid var(--ink-3)',
 fontSize: 13, lineHeight: 1.55
 }} className="p-3 bg-paper-2 text-ink-2" >
          No constructive pattern applies here — sensei recommends straight removal.
        </div>
      )}
    </>
  );
}

// ───────────────────────────────────────────────────────────
// Sessions tab — uses recentSessions
function ProjSessions() {
  return (
    <div className="py-6 px-12" >
      <SectionHeading k="録" label="Sessions in this project"
                      right={<span className="mono text-ink-3" style={{ fontSize: 11 }}>
                        28 in last 7d
                      </span>}/>
      <div className="flex flex-col" >
        {window.PROJECT_DATA.recentSessions.map(s => (
          <button key={s.id} style={{
 gridTemplateColumns: 'auto 120px 1fr auto auto auto' }} className="gap-4 py-3 px-1 grid items-center text-left border-b" >
            <span className="rounded-full" style={{ width: 8, height: 8,
 background: s.ftr ? 'var(--success)' : 'var(--warning)' }}/>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {s.project}
            </span>
            <span className="text-ink-2" style={{ fontSize: 13 }}>{s.title}</span>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {s.corrections === 0 ? "first-try" : `${s.corrections}×`}
            </span>
            <span className="mono text-ink-3 text-right" style={{ fontSize: 11,
 minWidth: 50 }}>{s.duration}</span>
            <span className="mono text-ink-4" style={{ fontSize: 11 }}>{s.time}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
// Settings pane — grouped (Variant A: card grid)
function ProjSettings({ project }) {
  const S = window.PROJECT_DATA.settings;
  return (
    <div style={{ gridTemplateColumns: '1fr 1fr' }} className="py-6 px-12 gap-8 grid items-start" >

      {/* Compact identity strip — full width */}
      <div style={{ gridColumn: '1 / -1' }}>
        <IdentityStrip project={project}/>
      </div>

      <SettingsCard title="Stack" action="+ add">
        <StackBlock project={project}/>
      </SettingsCard>

      <SettingsCard title="Repos" action="+ add repo">
        {project.repos.map(r => (
          <div key={r.id} style={{ gridTemplateColumns: '1fr auto' }} className="gap-2 py-2 px-0 grid items-baseline border-b" >
            <div>
              <div className="mono" style={{ fontSize: 13 }}>{r.id}</div>
              <div className="mono text-ink-4" style={{ fontSize: 11 }}>{r.path}</div>
            </div>
            <button className="text-ink-3" style={{ fontSize: 11 }}>remove</button>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Links" action="+ add link">
        {S.links.map(l => (
          <div key={l.id} style={{ gridTemplateColumns: '80px 1fr auto' }} className="gap-2 py-2 px-0 grid items-baseline border-b" >
            <span className="mono text-accent" style={{ fontSize: 11,
 letterSpacing: '0.1em' }}>{l.kind}</span>
            <div>
              <div style={{ fontSize: 13 }}>{l.label}</div>
              <div className="mono text-ink-4" style={{ fontSize: 11 }}>{l.url}</div>
            </div>
            <button className="text-ink-3" style={{ fontSize: 11 }}>edit</button>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Guidelines" action="+ add rule">
        {S.guidelines.map(g => (
          <div key={g.id} style={{
 fontSize: 13, lineHeight: 1.5
 }} className="py-2 px-0 border-b text-ink-2" >
            {g.rule}
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Backlog" action="+ add task">
        {S.backlog.map(b => (
          <div key={b.id} className="gap-2 py-2 px-0 flex items-baseline border-b" >
            <span style={{
 width: 6, height: 6 }} className="mt-1 rounded-full bg-ink-4" />
            <div className="flex-1 text-ink" style={{ fontSize: 13 }}>{b.task}</div>
            <span className="mono text-ink-4" style={{ fontSize: 11 }}>{b.added}</span>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Skills enabled">
        {S.skills.map(s => (
          <div key={s.id} style={{ gridTemplateColumns: '1fr auto' }} className="gap-2 py-2 px-0 grid items-center border-b" >
            <span style={{ fontSize: 13, color: s.on ? 'var(--ink)' : 'var(--ink-3)' }}>
              {s.name}
            </span>
            <ToggleChip on={s.on}/>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Excluded paths" action="+ add pattern">
        {S.excluded.map(p => (
          <div key={p} className="mono py-1 px-0 text-ink-2 border-b" style={{
 fontSize: 11 }}>{p}</div>
        ))}
      </SettingsCard>

      <SettingsCard title="Privacy">
        <ToggleRow label="Log prompts"            on={S.privacy.logPrompts}/>
        <ToggleRow label="Log file contents"      value={S.privacy.logFileContents}/>
        <ToggleRow label="Redact secrets"         on={S.privacy.redactSecrets}/>
        <ToggleRow label="Share with cloud"       on={S.privacy.shareWithCloud} tone="warn"/>
      </SettingsCard>
    </div>
  );
}

// Compact identity row — icon, name + inline client, one-line goal, row of actions
function IdentityStrip({ project }) {
  const icon = project.icon || { kind: "kanji", value: project.kanji, bg: 'var(--paper-3)', fg: 'var(--ink)' };
  return (
    <div style={{ gridTemplateColumns: '64px 1fr auto', borderRadius: 8
 }} className="gap-4 py-4 px-4 grid items-center bg-paper-2 border border-paper-edge" >
      {/* icon slot — swappable */}
      <button className="flex items-center justify-center relative" title="Change icon"
 style={{ width: 64, height: 64, borderRadius: 10,
 background: icon.bg, color: icon.fg,
 border: '1px solid var(--edge)' }}>
        <span className="kanji" style={{ fontSize: 40, lineHeight: 1 }}>{icon.value}</span>
        <span className="absolute rounded-full bg-paper border border-paper-edge text-ink-3 flex items-center justify-center" style={{ bottom: -6, right: -6,
 width: 20, height: 20,
 fontSize: 11 }}>
          ✎
        </span>
      </button>
      <div className="min-w-0" >
        <div className="gap-2 mb-1 flex items-baseline" >
          <div className="display font-normal" style={{ fontSize: 22, letterSpacing: '-0.01em' }}>
            {project.name}
          </div>
          <div className="text-ink-3" style={{ fontSize: 13 }}>· {project.client}</div>
        </div>
        <div className="text-ink-2 italic" style={{ fontSize: 13,
 lineHeight: 1.5, maxWidth: 560 }}>
          {project.goal}
        </div>
      </div>
      <button style={{
 fontSize: 11,
 borderRadius: 4 }} className="py-1 px-3 text-ink-3 border border-paper-edge bg-paper" >
        edit
      </button>
    </div>
  );
}

function StackBlock({ project }) {
  const S = project.stack || { languages: [], frameworks: [], runtimes: [], services: [] };
  const groups = [
    { label: "languages",  items: S.languages  },
    { label: "frameworks", items: S.frameworks },
    { label: "runtimes",   items: S.runtimes   },
    { label: "services",   items: S.services   }
  ].filter(g => g.items.length > 0);
  return (
    <div className="gap-2 flex flex-col" >
      {groups.map(g => (
        <div key={g.label} style={{ gridTemplateColumns: '80px 1fr' }} className="gap-2 py-1 px-0 grid items-baseline border-b" >
          <span className="mono text-ink-3 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
            {g.label}
          </span>
          <div className="gap-1 flex flex-wrap" >
            {g.items.map(it => (
              <span key={it} className="mono py-1 px-2 bg-paper border border-paper-edge text-ink-2" style={{
 fontSize: 11, borderRadius: 3 }}>
                {it}
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

// ─── Variant B: document-style settings with left summary rail ─────
// Less density-per-screen but much clearer hierarchy. Scrollable right
// column with anchor nav; sticky identity + stack summary on the left.
function ProjSettingsV2({ project }) {
  const S = window.PROJECT_DATA.settings;
  const stack = project.stack || { languages: [], frameworks: [], runtimes: [], services: [] };
  const icon = project.icon || { kind: "kanji", value: project.kanji, bg: 'var(--paper-3)', fg: 'var(--ink)' };
  const sections = ["identity","stack","repos","links","guidelines","backlog"];
  const [active, setActive] = React.useState("identity");

  return (
    <div style={{ gridTemplateColumns: '280px 1fr' }} className="gap-0 grid h-full bg-paper" >
      {/* Left rail — sticky summary */}
      <aside className="gap-4 pt-8 pb-6 px-6 border-r bg-paper-2 flex flex-col" >
        <button className="flex items-center justify-center self-start relative" title="Change icon"
 style={{ width: 80, height: 80, borderRadius: 12,
 background: icon.bg, color: icon.fg,
 border: '1px solid var(--edge)' }}>
          <span className="kanji" style={{ fontSize: 40 }}>{icon.value}</span>
          <span className="absolute rounded-full bg-paper border border-paper-edge text-ink-3 flex items-center justify-center" style={{ bottom: -7, right: -7,
 width: 22, height: 22,
 fontSize: 11 }}>✎</span>
        </button>
        <div>
          <div className="display font-normal" style={{ fontSize: 22,
 letterSpacing: '-0.01em', lineHeight: 1.15 }}>
            {project.name}
          </div>
          <div style={{ fontSize: 13 }} className="mt-1 text-ink-3" >
            {project.client}
          </div>
          <div style={{
 fontSize: 13,
 lineHeight: 1.5 }} className="mt-2 text-ink-2 italic" >
            {project.goal}
          </div>
        </div>

        <div style={{ height: 1, background: 'var(--edge)' }}/>

        {/* quick facts */}
        <div className="gap-2 flex flex-col" >
          <QuickFact label="repos"   value={project.repos.length}/>
          <QuickFact label="skills"  value={S.skills.filter(s=>s.on).length + " of " + S.skills.length}/>
          <QuickFact label="links"   value={S.links.length}/>
          <QuickFact label="backlog" value={S.backlog.length}/>
        </div>

        <div style={{ height: 1, background: 'var(--edge)' }}/>

        {/* anchor nav */}
        <nav style={{ marginTop: 'auto' }} className="gap-1 flex flex-col" >
          {sections.map(id => (
            <button key={id} onClick={() => setActive(id)}
 style={{
 fontSize: 13, color: active===id ? 'var(--ink)' : 'var(--ink-3)',
 background: active===id ? 'var(--paper)' : 'transparent',
 borderRadius: 4,
 borderLeft: active===id ? '2px solid var(--accent)' : '2px solid transparent',
 fontWeight: active===id ? 500 : 400 }} className="py-2 px-2 text-left capitalize" >
              {id}
            </button>
          ))}
        </nav>
      </aside>

      {/* Right — document */}
      <div style={{ maxHeight: '100%' }} className="py-8 px-12 overflow-auto" >
        <V2Block id="identity" title="Identity" desc="The human-readable face of this project.">
          <V2Field label="Name"   value={project.name}/>
          <V2Field label="Client" value={project.client}/>
          <V2Field label="Goal"   value={project.goal} multiline/>
          <V2Field label="Icon"   value={`kanji · ${icon.value}`} action="change"/>
        </V2Block>

        <V2Block id="stack" title="Stack"
                 desc="Drives MCP recommendations and helps sensei reason about your code. Edit anytime.">
          {[["Languages", stack.languages],
            ["Frameworks", stack.frameworks],
            ["Runtimes", stack.runtimes],
            ["Services", stack.services]].map(([label, items]) => (
              <div key={label} style={{ gridTemplateColumns: '120px 1fr auto' }} className="gap-3 py-2 px-0 grid items-center border-b" >
                <span className="mono text-ink-3 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>{label}</span>
                <div className="gap-1 flex flex-wrap" >
                  {items.length > 0 ? items.map(it => (
                    <span key={it} className="mono py-1 px-2 bg-paper-2 border border-paper-edge text-ink-2" style={{
 fontSize: 11, borderRadius: 3 }}>{it}</span>
                  )) : <span className="text-ink-4 italic" style={{ fontSize: 11 }}>none</span>}
                </div>
                <button className="text-accent" style={{ fontSize: 11 }}>+ add</button>
              </div>
            ))}
        </V2Block>

        <V2Block id="repos" title="Repos" desc="Folders sensei watches for this project.">
          {project.repos.map(r => (
            <div key={r.id} style={{ gridTemplateColumns: '1fr auto auto' }} className="gap-3 py-3 px-0 grid items-baseline border-b" >
              <div>
                <div className="text-ink" style={{ fontSize: 13 }}>{r.id}</div>
                <div className="mono mt-1 text-ink-4" style={{ fontSize: 11 }}>
                  {r.path}
                </div>
              </div>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                {r.size} · {r.lang}
              </span>
              <button className="text-ink-3" style={{ fontSize: 11 }}>remove</button>
            </div>
          ))}
          <button style={{ fontSize: 13 }} className="mt-2 text-accent" >+ add repo</button>
        </V2Block>


        <V2Block id="links" title="Links" desc="Docs, dashboards, runbooks — anything sensei should remember.">
          {S.links.map(l => (
            <div key={l.id} style={{ gridTemplateColumns: '90px 1fr auto' }} className="gap-3 py-3 px-0 grid items-baseline border-b" >
              <span className="mono text-accent" style={{ fontSize: 11,
 letterSpacing: '0.1em' }}>{l.kind}</span>
              <div>
                <div style={{ fontSize: 13 }}>{l.label}</div>
                <div className="mono mt-1 text-ink-4" style={{ fontSize: 11 }}>{l.url}</div>
              </div>
              <button className="text-ink-3" style={{ fontSize: 11 }}>edit</button>
            </div>
          ))}
          <button style={{ fontSize: 13 }} className="mt-2 text-accent" >+ add link</button>
        </V2Block>

        <V2Block id="guidelines" title="Guidelines"
                 desc="Rules assistants should follow when working on this project.">
          {S.guidelines.map(g => (
            <div key={g.id} style={{
 fontSize: 13, lineHeight: 1.55
 }} className="py-3 px-0 border-b text-ink-2" >
              {g.rule}
            </div>
          ))}
          <button style={{ fontSize: 13 }} className="mt-2 text-accent" >+ add rule</button>
        </V2Block>

        <V2Block id="backlog" title="Backlog"
                 desc="Things sensei should surface when relevant.">
          {S.backlog.map(b => (
            <div key={b.id} style={{ gridTemplateColumns: '12px 1fr auto' }} className="gap-3 py-3 px-0 grid items-baseline border-b" >
              <span style={{
 width: 6, height: 6 }} className="mt-1 rounded-full bg-ink-4" />
              <div className="text-ink" style={{ fontSize: 13 }}>{b.task}</div>
              <span className="mono text-ink-4" style={{ fontSize: 11 }}>{b.added}</span>
            </div>
          ))}
          <button style={{ fontSize: 13 }} className="mt-2 text-accent" >+ add task</button>
        </V2Block>
      </div>
    </div>
  );
}

function V2Block({ id, title, desc, children }) {
  return (
    <section id={id}
             style={{
                       borderBottom: '1px solid var(--edge)'
}} className="mb-8 pb-8" >
      <div className="mb-4" >
        <h2 className="display m-0 font-normal" style={{
 fontSize: 22,
 letterSpacing: '-0.01em'
 }}>
          {title}
        </h2>
        {desc && (
          <p style={{
 fontSize: 13, maxWidth: 560, lineHeight: 1.5
 }} className="mt-1 mb-0 text-ink-3" >
            {desc}
          </p>
        )}
      </div>
      <div>{children}</div>
    </section>
  );
}

function V2Field({ label, value, multiline, action }) {
  return (
    <div style={{ gridTemplateColumns: '120px 1fr auto', alignItems: multiline ? 'flex-start' : 'baseline' }} className="gap-3 py-3 px-0 grid border-b" >
      <span className="mono text-ink-3 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>{label}</span>
      <div className="text-ink" style={{ fontSize: 13,
 fontStyle: multiline ? 'italic' : 'normal',
 lineHeight: multiline ? 1.55 : 1.4 }}>
        {value}
      </div>
      <button style={{ fontSize: 11, color: action ? 'var(--accent)' : 'var(--ink-4)' }}>
        {action || "edit"}
      </button>
    </div>
  );
}

function QuickFact({ label, value }) {
  return (
    <div className="flex justify-between items-baseline" >
      <span className="mono text-ink-3 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.1em' }}>{label}</span>
      <span className="mono text-ink" style={{ fontSize: 13 }}>{value}</span>
    </div>
  );
}

function SettingsCard({ title, action, children }) {
  return (
    <div style={{
 borderRadius: 8
 }} className="py-4 px-4 bg-paper-2 border border-paper-edge" >
      <div className="mb-2 flex items-baseline justify-between" >
        <h3 className="display m-0 font-normal" style={{
 fontSize: 13,
 letterSpacing: '0.01em'
 }}>{title}</h3>
        {action && (
          <button className="text-accent" style={{ fontSize: 11 }}>{action}</button>
        )}
      </div>
      {children}
    </div>
  );
}
function Field({ label, value, multiline }) {
  return (
    <div className="py-2 px-0 border-b" >
      <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{label}</div>
      <div style={{
 fontSize: 13,
                     fontStyle: multiline ? 'italic' : 'normal',
                     color: multiline ? 'var(--ink-2)' : 'var(--ink)'
}} className="mt-1" >{value}</div>
    </div>
  );
}
function ToggleChip({ on }) {
  return (
    <span className="mono py-1 px-2" style={{
      fontSize: 11, borderRadius: 3,
      background: on ? 'var(--success-soft)' : 'var(--paper-3)',
      color: on ? 'var(--success)' : 'var(--ink-3)'
}}>{on ? "on" : "off"}</span>
  );
}
function ToggleRow({ label, on, value, tone }) {
  return (
    <div style={{ gridTemplateColumns: '1fr auto' }} className="gap-2 py-2 px-0 grid items-center border-b" >
      <span style={{ fontSize: 13, color: tone === "warn" ? 'var(--warning)' : 'var(--ink)' }}>
        {label}
      </span>
      {value ? (
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{value}</span>
      ) : (
        <ToggleChip on={on}/>
      )}
    </div>
  );
}

// ───────────────────────────────────────────────────────────
// Action drawer — quick send + advanced palette
function ProjActionDrawer({ rec, mode, onClose }) {
  const [text, setText] = pS(rec.prompt);
  const [acp, setAcp] = pS(rec.defaultAcp);
  const acps = [
    { id: "claude-code", label: "Claude Code", sub: "cli · claude" },
    { id: "cursor",      label: "Cursor",      sub: "editor" },
    { id: "codex",       label: "Codex CLI",   sub: "openai" },
    { id: "aider",       label: "Aider",       sub: "cli · aider" },
    { id: "copy",        label: "Copy prompt", sub: "clipboard" }
  ];
  return (
    <div className="absolute flex justify-end" style={{ inset: 0, background: 'var(--scrim)', zIndex: 50
 }} onClick={onClose}>
      <div className="bg-paper shadow-lg flex flex-col min-h-0" onClick={(e) => e.stopPropagation()}
 style={{ width: 520 }}>
        <div className="gap-3 pt-6 pb-3 px-6 border-b flex items-start" >
          <span className="kanji text-accent" style={{ fontSize: 28, lineHeight: 1 }}>
            {rec.kanji || "送"}
          </span>
          <div className="flex-1" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-1 text-ink-3 uppercase" >
              {mode === "send" ? "Send prompt" : "Customize prompt"}
            </div>
            <div className="display font-normal" style={{ fontSize: 17, letterSpacing: '-0.005em' }}>
              {rec.promptTitle || rec.title}
            </div>
            <div style={{ fontSize: 11 }} className="mono mt-1 text-ink-3">
              cwd · {rec.cwd || "—"}
            </div>
          </div>
          <button className="text-ink-3" onClick={onClose} style={{ fontSize: 13 }}>✕</button>
        </div>

        {/* ACP picker */}
        <div className="py-3 px-6 border-b" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >Send to</div>
          <div className="gap-1 flex flex-wrap" >
            {acps.map(a => {
              const on = acp === a.id;
              return (
                <button key={a.id} onClick={() => setAcp(a.id)}
 style={{
 borderRadius: 5, fontSize: 11,
 background: on ? 'var(--ink)' : 'var(--paper-2)',
 color: on ? 'var(--paper)' : 'var(--ink)',
 border: on ? 'none' : 'var(--hairline)'
 }} className="py-2 px-3 text-left" >
                  <div className="font-medium" >{a.label}</div>
                  <div className="mono" style={{ fontSize: 11, opacity: 0.6 }}>{a.sub}</div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Prompt editor */}
        <div className="py-3 px-6 flex-1 overflow-auto" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
            {mode === "send" ? "Prompt · preview" : "Prompt · editable"}
          </div>
          <textarea
 value={text}
 onChange={(e) => setText(e.target.value)}
 readOnly={mode === "send"}
 style={{ minHeight: 300,
 fontFamily: 'var(--font-mono)', fontSize: 13, lineHeight: 1.6,
 borderRadius: 6, resize: 'vertical'
 }} className="p-3 w-full bg-paper-2 border border-paper-edge text-ink" />

          {rec.evidence && (
            <div style={{ fontSize: 11 }} className="mt-3 text-ink-3" >
              <div style={{ letterSpacing: '0.14em' }} className="mb-1 uppercase" >
                Evidence attached
              </div>
              <div className="mono">{rec.evidence.join(" · ")}</div>
            </div>
          )}
        </div>

        {/* Footer actions */}
        <div className="py-3 px-6 gap-2 border-t flex items-center" >
          <span className="text-ink-3" style={{ fontSize: 11 }}>
            {text.length.toLocaleString()} chars · will launch in {acp}
          </span>
          <span className="flex-1" />
          <button onClick={onClose}
 style={{
 fontSize: 13, border: 'var(--ink-line)', borderRadius: 5
 }} className="py-2 px-3 text-ink-2" >
            cancel
          </button>
          <button style={{
 fontSize: 13, borderRadius: 5
 }} className="py-2 px-4 bg-ink text-paper" >
            {acp === "copy" ? "copy to clipboard" : `launch ${acp} →`}
          </button>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  ProjHeader, ProjOverview, ProjGraphLens, ProjPatterns, ProjSessions, ProjSettings, ProjSettingsV2,
  ProjActionDrawer, ProjRecCard, SectionHeading, ProjMiniSpark
});
