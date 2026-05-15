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
    <svg width={w} height={h} style={{ color, display: 'block' }}>
      <path d={d} className="sparkline-path"/>
      <circle cx={pts[pts.length-1][0]} cy={pts[pts.length-1][1]} r={2} fill="currentColor"/>
    </svg>
  );
}

// ───────────────────────────────────────────────────────────
// Project header — used at the top of every layout
function ProjHeader({ project, onBack, showBack = true }) {
  return (
    <div style={{ padding: '24px 48px 16px', borderBottom: 'var(--hairline)',
                  display: 'flex', alignItems: 'flex-start', gap: 24, background: 'var(--paper)' }}>
      {showBack && (
        <button onClick={onBack}
                style={{ fontSize: 11, color: 'var(--ink-3)',
                         padding: '4px 8px', border: 'var(--ink-line)', borderRadius: 5 }}>
          ← all projects
        </button>
      )}
      <div className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1, marginTop: -4 }}>
        {project.kanji}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                      textTransform: 'uppercase', marginBottom: 4 }}>
          Project · {project.client}
        </div>
        <h1 className="display" style={{ fontSize: 28, fontWeight: 400, margin: '0 0 4px',
                                          letterSpacing: '-0.01em' }}>
          {project.name}
        </h1>
        <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: 0, maxWidth: 560,
                    fontStyle: 'italic', lineHeight: 1.5 }}>
          "{project.goal}"
        </p>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 24,
                    padding: '4px 0', color: 'var(--ink-2)' }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>FTR · 14d</div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 4 }}>
            <span className="display" style={{ fontSize: 22, fontWeight: 400 }}>
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
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>Sessions · 7d</div>
          <div className="display" style={{ fontSize: 22, fontWeight: 400 }}>{project.sessions7d}</div>
        </div>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>Preferred ACP</div>
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
    <div style={{ display: 'grid', gridTemplateColumns: '1.35fr 1fr', gap: 32, padding: '24px 48px' }}>
      <div>
        <SectionHeading k="紋" label="Repos in this project"/>
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {project.repos.map(r => (
            <div key={r.id} style={{
              display: 'grid', gridTemplateColumns: '1fr auto auto', gap: 12, alignItems: 'baseline',
              padding: '12px 4px', borderBottom: 'var(--hairline)'
            }}>
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{r.id}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{r.path}</div>
              </div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{r.lang}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{r.size}</div>
            </div>
          ))}
        </div>

        <div style={{ height: 32 }}/>
        <SectionHeading k="師" label="What sensei recommends"
                        right={<span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                          {recs.length} open
                        </span>}/>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {recs.map(r => <ProjRecCard key={r.id} rec={r} openAction={openAction}/>)}
        </div>
      </div>

      <div>
        <SectionHeading k="急" label="Hotspots"/>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 24 }}>
          {D.files.filter(f => f.tags.includes("hot") || f.tags.includes("god-node")).slice(0, 5).map(f => (
            <div key={f.path} style={{
              display: 'grid', gridTemplateColumns: '1fr auto', gap: 8, alignItems: 'baseline',
              padding: '8px 4px', borderBottom: 'var(--hairline)'
            }}>
              <div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink)' }}>{f.path}</div>
                <div style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
                  {f.repo} · rework {f.rework}× {f.tags.length ? "· " + f.tags.join(", ") : ""}
                </div>
              </div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--warning)' }}>
                {f.rework}×
              </span>
            </div>
          ))}
        </div>

        <SectionHeading k="紋" label="Patterns in use"/>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {D.patterns.followed.slice(0, 4).map(p => (
            <div key={p.id} style={{
              padding: '8px 4px', borderBottom: 'var(--hairline)'
            }}>
              <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between' }}>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{p.name}</div>
                <span className="mono" style={{ fontSize: 11,
                      color: p.status === "rule" ? 'var(--success)' :
                             p.status === "gap" ? 'var(--warning)' : 'var(--ink-3)' }}>
                  {p.status}
                </span>
              </div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
                {p.family} · {p.places} places
              </div>
            </div>
          ))}
        </div>
        {D.patterns.antiPatterns.length > 0 && (
          <div style={{ marginTop: 12, padding: '8px 12px',
                         background: 'var(--warning-soft)', borderLeft: '2px solid var(--warning)',
                         borderRadius: 5, fontSize: 11, color: 'var(--ink-2)',
                         lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--warning)',
                          marginRight: 4 }}>避</span>
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
    <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                  marginBottom: 12 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>{k}</span>
        <h2 className="display" style={{ fontSize: 15, fontWeight: 400, margin: 0 }}>{label}</h2>
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
      padding: '12px 16px 12px 16px', borderRadius: 6,
      background: 'var(--paper-2)', border: 'var(--hairline)',
      borderLeft: `2px solid ${tone}`, display: 'grid',
      gridTemplateColumns: 'auto 1fr auto', gap: 12, alignItems: 'start'
    }}>
      <span className="kanji" style={{ fontSize: 22, color: tone, lineHeight: 1 }}>{rec.kanji}</span>
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)', marginBottom: 4, lineHeight: 1.45 }}>
          {rec.title}
        </div>
        <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }}>
          {rec.why}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 8,
                      fontSize: 11, color: 'var(--ink-3)' }} className="mono">
          <span style={{ color: 'var(--accent)' }}>· {rec.impact}</span>
          <span>· {rec.evidence.join(" · ")}</span>
        </div>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        <button onClick={() => openAction(rec, "send")}
                style={{ padding: '8px 12px', fontSize: 11,
                         background: 'var(--ink)', color: 'var(--paper)',
                         borderRadius: 5, whiteSpace: 'nowrap' }}>
          send to {rec.defaultAcp} →
        </button>
        <button onClick={() => openAction(rec, "palette")}
                style={{ padding: '4px 12px', fontSize: 11,
                         color: 'var(--ink-2)', border: 'var(--ink-line)',
                         borderRadius: 5, whiteSpace: 'nowrap' }}>
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
    <div style={{ padding: '24px 48px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                    marginBottom: 16 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
          <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)' }}>構</span>
          <h2 className="display" style={{ fontSize: 17, fontWeight: 400, margin: 0 }}>
            Code visualization
          </h2>
          <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            — three lenses on the same graph
          </span>
        </div>
        <div style={{ display: 'flex', gap: 4, padding: 4, background: 'var(--paper-3)', borderRadius: 6 }}>
          {[["graph","Call graph"],["matrix","Matrix"],["hairball","Clusters"]].map(([id, lbl]) => (
            <button key={id} onClick={() => setLens(id)}
                    style={{ padding: '4px 12px', fontSize: 11,
                             borderRadius: 4,
                             background: lens === id ? 'var(--paper)' : 'transparent',
                             color: lens === id ? 'var(--ink)' : 'var(--ink-3)' }}>
              {lbl}
            </button>
          ))}
        </div>
      </div>

      {/* Overlay chips */}
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 12 }}>
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
                    style={{
                      display: 'inline-flex', alignItems: 'center', gap: 8,
                      padding: '4px 12px', fontSize: 11,
                      borderRadius: 999,
                      background: on ? 'var(--accent-soft)' : 'var(--paper-2)',
                      border: on ? '1px solid transparent' : 'var(--hairline)',
                      color: on ? 'var(--accent)' : 'var(--ink-2)'
                    }}>
              <span className="kanji" style={{ fontSize: 11 }}>{k}</span>
              {lbl}
            </button>
          );
        })}
      </div>

      <div style={{ position: 'relative', background: 'var(--paper-2)',
                    border: 'var(--hairline)', borderRadius: 10, overflow: 'hidden' }}>
        {lens === "graph"    && <LensGraph    D={D} overlay={overlay}/>}
        {lens === "matrix"   && <LensMatrix   D={D} overlay={overlay}/>}
        {lens === "hairball" && <LensHairball D={D} overlay={overlay}/>}
      </div>

      {/* Legend / selected detail */}
      {overlay === "duplicates" && D.duplicates.length > 0 && (
        <div style={{ marginTop: 12, padding: '12px 12px',
                      background: 'var(--paper-2)', border: 'var(--hairline)',
                      borderLeft: '2px solid var(--warning)', borderRadius: 6,
                      fontSize: 13, color: 'var(--ink-2)' }}>
          <div style={{ display: 'flex', gap: 8, alignItems: 'baseline' }}>
            <span className="kanji" style={{ color: 'var(--warning)', fontSize: 15 }}>双</span>
            <b style={{ color: 'var(--ink)', fontWeight: 500 }}>{D.duplicates[0].title}</b>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              confidence {Math.round(D.duplicates[0].confidence * 100)}%
            </span>
          </div>
          <div style={{ marginTop: 4, color: 'var(--ink-3)' }}>
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
    <div style={{ padding: 16 }}>
      <svg width={W} height={H} style={{ display: 'block' }}>
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
      <div style={{ display: 'flex', justifyContent: 'space-between',
                    marginTop: 4, fontSize: 11, color: 'var(--ink-4)' }} className="mono">
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
    <div style={{ padding: '16px 24px', display: 'grid',
                  gridTemplateColumns: 'repeat(6, 1fr)', gap: 8 }}>
      {sorted.map(n => {
        const v = metric(n);
        return (
          <div key={n.id} style={{
            padding: '12px 8px', borderRadius: 5,
            background: colorFor(v),
            border: 'var(--hairline)',
            minHeight: 76, display: 'flex', flexDirection: 'column', justifyContent: 'space-between'
          }}>
            <div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink)',
                              wordBreak: 'break-all', lineHeight: 1.3 }}>
                {n.id}
              </div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
                {n.repo}
              </div>
            </div>
            <div className="mono" style={{ fontSize: 13, fontWeight: 500, color: 'var(--ink)',
                          textAlign: 'right' }}>
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
    <div style={{ padding: 16 }}>
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
    <div style={{ padding: '24px 48px' }}>
      {/* Toggle */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <div style={{ display: 'flex', background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 6, padding: 4 }}>
          <PatSideBtn on={side === "follow"} onClick={() => setSide("follow")}
                      kanji="紋" label="Patterns in use" count={P.followed.length}/>
          <PatSideBtn on={side === "avoid"} onClick={() => setSide("avoid")}
                      kanji="避" label="Anti-patterns" count={P.antiPatterns.length}
                      warn/>
        </div>
        <span style={{ fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.5, maxWidth: 440 }}>
          {side === "follow"
            ? "Constructive patterns sensei detects across your code — promoted to rules once adopted."
            : "Duplication, god-nodes, monoliths. Where fixable, sensei suggests a pattern that would resolve it."}
        </span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.1fr', gap: 32 }}>
        {/* LEFT — list */}
        <div style={{ display: 'flex', flexDirection: 'column' }}>
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
              padding: '8px 12px', fontSize: 13,
              display: 'inline-flex', gap: 8, alignItems: 'center', borderRadius: 4,
              background: on ? 'var(--ink)' : 'transparent',
              color: on ? 'var(--paper)' : 'var(--ink-2)'
            }}>
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
            style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 12,
              alignItems: 'start', padding: '12px 12px', textAlign: 'left',
              borderBottom: 'var(--hairline)',
              background: on ? 'var(--paper-2)' : 'transparent'
            }}>
      <span className="kanji" style={{ fontSize: 17, color: tone }}>{p.kanji}</span>
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>{p.name}</div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
          {p.family} · {p.places} places · {p.recent}
        </div>
      </div>
      <span className="mono" style={{ fontSize: 11, color: tone,
                    background: bg, padding: '4px 8px', borderRadius: 3 }}>
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
            style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 12,
              alignItems: 'start', padding: '12px 12px', textAlign: 'left',
              borderBottom: 'var(--hairline)',
              background: on ? 'var(--paper-2)' : 'transparent'
            }}>
      <span className="kanji" style={{ fontSize: 17, color: sevTone }}>{a.kanji}</span>
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>{a.name}</div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
          {a.type} · {a.occurrences}× · {a.suggest ? `fix: ${a.suggest.name}` : "no pattern suggested"}
        </div>
      </div>
      <span className="mono" style={{ fontSize: 11, color: sevTone,
                    background: sevBg, padding: '4px 8px', borderRadius: 3 }}>
        {a.severity}
      </span>
    </button>
  );
}

function FollowDetail({ p, openAction }) {
  return (
    <>
      <SectionHeading k={p.kanji} label={p.name}/>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', marginBottom: 8,
                     letterSpacing: '0.06em', textTransform: 'uppercase' }}>
        {p.family}
      </div>
      <div style={{ fontSize: 13, color: 'var(--ink-2)', marginBottom: 12, lineHeight: 1.55 }}>
        {p.summary}
      </div>
      <div style={{ fontSize: 13, color: 'var(--ink-3)', marginBottom: 12, lineHeight: 1.6 }}>
        Detected in <span className="mono" style={{ color: 'var(--ink)' }}>{p.places}</span> places.
        Confidence <span className="mono" style={{ color: 'var(--ink)' }}>{Math.round(p.confidence * 100)}%</span>.
        First seen in <span className="mono">{p.file}</span>.
      </div>
      <div style={{ padding: '12px 16px', background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 6,
                     fontFamily: 'var(--font-mono)', fontSize: 13,
                     color: 'var(--ink)', whiteSpace: 'pre-wrap',
                     marginBottom: 16 }}>
        {p.example}
      </div>
      {p.enforcement && (
        <div style={{ padding: '8px 12px', background: 'var(--success-soft)', borderRadius: 6,
                       borderLeft: '2px solid var(--success)', fontSize: 13,
                       color: 'var(--ink-2)', marginBottom: 12 }}>
          {p.enforcement}
        </div>
      )}
      {p.status === "gap" && (
        <div style={{ padding: 12, background: 'var(--warning-soft)', borderRadius: 6,
                       borderLeft: '2px solid var(--warning)',
                       display: 'flex', gap: 12, alignItems: 'center' }}>
          <div style={{ flex: 1, fontSize: 13, color: 'var(--ink-2)' }}>
            This pattern is recommended but missing. Adopt it as a project rule?
          </div>
          <button onClick={() => openAction({
            id: "ad-hoc",
            defaultAcp: "claude-code",
            promptTitle: `Adopt pattern: ${p.name}`,
            prompt: `Adopt pattern "${p.name}" as a project rule.\n\n${p.summary}\n\nGenerate .sensei/rules/${p.id}.md.`
          }, "palette")}
                  style={{ padding: '8px 12px', fontSize: 11,
                           background: 'var(--ink)', color: 'var(--paper)',
                           borderRadius: 5, whiteSpace: 'nowrap' }}>
            Adopt →
          </button>
        </div>
      )}
      {p.status === "suggested" && (
        <div style={{ padding: 12, background: 'var(--accent-soft)', borderRadius: 6,
                       borderLeft: '2px solid var(--accent)',
                       display: 'flex', gap: 12, alignItems: 'center' }}>
          <div style={{ flex: 1, fontSize: 13, color: 'var(--ink-2)' }}>
            Emerging pattern — appears in {p.places} places but not yet a project rule. Promote?
          </div>
          <button onClick={() => openAction({
            id: "ad-hoc",
            defaultAcp: "claude-code",
            promptTitle: `Promote pattern: ${p.name}`,
            prompt: `Promote pattern "${p.name}" to a project rule.\n\n${p.summary}\n\nExample:\n${p.example}`
          }, "palette")}
                  style={{ padding: '8px 12px', fontSize: 11,
                           background: 'var(--ink)', color: 'var(--paper)',
                           borderRadius: 5, whiteSpace: 'nowrap' }}>
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
      <div style={{ display: 'flex', gap: 8, marginBottom: 12 }}>
        <span className="mono" style={{ fontSize: 11, color: sevTone,
                      padding: '4px 8px', borderRadius: 3,
                      background: a.severity === "high" ? 'var(--accent-soft)' :
                                  a.severity === "medium" ? 'var(--warning-soft)' : 'var(--paper-3)' }}>
          {a.severity} · {a.type}
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                      padding: '4px 8px', borderRadius: 3, background: 'var(--paper-3)' }}>
          {a.occurrences}× occurrences
        </span>
      </div>
      <div style={{ fontSize: 13, color: 'var(--ink-2)', marginBottom: 12, lineHeight: 1.55 }}>
        {a.summary}
      </div>

      {/* Occurrence list */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 8 }}>
          Where
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4,
                       padding: '8px 8px', background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 6 }}>
          {a.places.map((p, i) => (
            <div key={i} className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
                          padding: '4px 0' }}>
              · {p}
            </div>
          ))}
        </div>
      </div>

      <div style={{ padding: '12px 16px', background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 6,
                     fontFamily: 'var(--font-mono)', fontSize: 13,
                     color: 'var(--ink)', whiteSpace: 'pre-wrap',
                     marginBottom: 16 }}>
        {a.example}
      </div>

      {/* Suggested fix cross-link */}
      {a.suggest && (
        <div style={{ padding: 12, background: 'var(--success-soft)', borderRadius: 6,
                       borderLeft: '2px solid var(--success)' }}>
          <div style={{ display: 'flex', gap: 8, alignItems: 'baseline', marginBottom: 4 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--success)' }}>紋</span>
            <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                           textTransform: 'uppercase' }}>Suggested pattern</span>
            <span style={{ fontSize: 13, color: 'var(--ink)' }}>{a.suggest.name}</span>
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55,
                         marginBottom: 8 }}>
            {a.suggest.reason}
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            {allFollowed.find(f => f.id === a.suggest.patternId) && (
              <button onClick={() => jumpToFollowed(a.suggest.patternId)}
                      style={{ padding: '4px 12px', fontSize: 11,
                                background: 'var(--paper)', color: 'var(--ink-2)',
                                borderRadius: 4, border: 'var(--hairline)' }}>
                See {a.suggest.name} →
              </button>
            )}
            <button onClick={() => openAction({
              id: "ad-hoc",
              defaultAcp: "claude-code",
              promptTitle: `Refactor: ${a.name}`,
              prompt: `Refactor "${a.name}" using the ${a.suggest.name} pattern.\n\n${a.suggest.reason}\n\nSites:\n${a.places.map(x => "  - " + x).join("\n")}`
            }, "palette")}
                    style={{ padding: '4px 12px', fontSize: 11,
                              background: 'var(--ink)', color: 'var(--paper)',
                              borderRadius: 4 }}>
              Refactor with this pattern →
            </button>
          </div>
        </div>
      )}

      {!a.suggest && (
        <div style={{ padding: 12, background: 'var(--paper-2)', borderRadius: 6,
                       borderLeft: '2px solid var(--ink-3)',
                       fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }}>
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
    <div style={{ padding: '24px 48px' }}>
      <SectionHeading k="録" label="Sessions in this project"
                      right={<span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                        28 in last 7d
                      </span>}/>
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {window.PROJECT_DATA.recentSessions.map(s => (
          <button key={s.id} style={{
            display: 'grid',
            gridTemplateColumns: 'auto 120px 1fr auto auto auto',
            gap: 16, alignItems: 'center',
            padding: '12px 4px', textAlign: 'left',
            borderBottom: 'var(--hairline)'
          }}>
            <span style={{ width: 8, height: 8, borderRadius: '50%',
                            background: s.ftr ? 'var(--success)' : 'var(--warning)' }}/>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {s.project}
            </span>
            <span style={{ fontSize: 13, color: 'var(--ink-2)' }}>{s.title}</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {s.corrections === 0 ? "first-try" : `${s.corrections}×`}
            </span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                          minWidth: 50, textAlign: 'right' }}>{s.duration}</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{s.time}</span>
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
    <div style={{ padding: '24px 48px',
                  display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 32, alignItems: 'start' }}>

      {/* Compact identity strip — full width */}
      <div style={{ gridColumn: '1 / -1' }}>
        <IdentityStrip project={project}/>
      </div>

      <SettingsCard title="Stack" action="+ add">
        <StackBlock project={project}/>
      </SettingsCard>

      <SettingsCard title="Repos" action="+ add repo">
        {project.repos.map(r => (
          <div key={r.id} style={{ display: 'grid', gridTemplateColumns: '1fr auto',
                                    gap: 8, alignItems: 'baseline',
                                    padding: '8px 0', borderBottom: 'var(--hairline)' }}>
            <div>
              <div className="mono" style={{ fontSize: 13 }}>{r.id}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{r.path}</div>
            </div>
            <button style={{ fontSize: 11, color: 'var(--ink-3)' }}>remove</button>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Links" action="+ add link">
        {S.links.map(l => (
          <div key={l.id} style={{ display: 'grid', gridTemplateColumns: '80px 1fr auto',
                                    gap: 8, alignItems: 'baseline',
                                    padding: '8px 0', borderBottom: 'var(--hairline)' }}>
            <span className="mono" style={{ fontSize: 11, color: 'var(--accent)',
                          letterSpacing: '0.1em' }}>{l.kind}</span>
            <div>
              <div style={{ fontSize: 13 }}>{l.label}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{l.url}</div>
            </div>
            <button style={{ fontSize: 11, color: 'var(--ink-3)' }}>edit</button>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Guidelines" action="+ add rule">
        {S.guidelines.map(g => (
          <div key={g.id} style={{ padding: '8px 0', borderBottom: 'var(--hairline)',
                                    fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.5 }}>
            {g.rule}
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Backlog" action="+ add task">
        {S.backlog.map(b => (
          <div key={b.id} style={{ display: 'flex', alignItems: 'baseline', gap: 8,
                                    padding: '8px 0', borderBottom: 'var(--hairline)' }}>
            <span style={{ width: 6, height: 6, borderRadius: '50%',
                            background: 'var(--ink-4)', marginTop: 4 }}/>
            <div style={{ flex: 1, fontSize: 13, color: 'var(--ink)' }}>{b.task}</div>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{b.added}</span>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Skills enabled">
        {S.skills.map(s => (
          <div key={s.id} style={{ display: 'grid', gridTemplateColumns: '1fr auto',
                                    gap: 8, alignItems: 'center',
                                    padding: '8px 0', borderBottom: 'var(--hairline)' }}>
            <span style={{ fontSize: 13, color: s.on ? 'var(--ink)' : 'var(--ink-3)' }}>
              {s.name}
            </span>
            <ToggleChip on={s.on}/>
          </div>
        ))}
      </SettingsCard>

      <SettingsCard title="Excluded paths" action="+ add pattern">
        {S.excluded.map(p => (
          <div key={p} className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
                        padding: '4px 0', borderBottom: 'var(--hairline)' }}>{p}</div>
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
    <div style={{ display: 'grid', gridTemplateColumns: '64px 1fr auto',
                   gap: 16, alignItems: 'center',
                   padding: '16px 16px',
                   background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 8 }}>
      {/* icon slot — swappable */}
      <button title="Change icon"
              style={{ width: 64, height: 64, borderRadius: 10,
                        background: icon.bg, color: icon.fg,
                        display: 'flex', alignItems: 'center', justifyContent: 'center',
                        border: '1px solid var(--edge)',
                        position: 'relative' }}>
        <span className="kanji" style={{ fontSize: 40, lineHeight: 1 }}>{icon.value}</span>
        <span style={{ position: 'absolute', bottom: -6, right: -6,
                        width: 20, height: 20, borderRadius: '50%',
                        background: 'var(--paper)', border: 'var(--hairline)',
                        fontSize: 11, color: 'var(--ink-3)',
                        display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          ✎
        </span>
      </button>
      <div style={{ minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 4 }}>
          <div className="display" style={{ fontSize: 22, fontWeight: 400, letterSpacing: '-0.01em' }}>
            {project.name}
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-3)' }}>· {project.client}</div>
        </div>
        <div style={{ fontSize: 13, color: 'var(--ink-2)', fontStyle: 'italic',
                       lineHeight: 1.5, maxWidth: 560 }}>
          {project.goal}
        </div>
      </div>
      <button style={{ fontSize: 11, color: 'var(--ink-3)',
                        padding: '4px 12px', border: 'var(--hairline)',
                        borderRadius: 4, background: 'var(--paper)' }}>
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
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {groups.map(g => (
        <div key={g.label} style={{ display: 'grid', gridTemplateColumns: '80px 1fr',
                                     gap: 8, alignItems: 'baseline',
                                     padding: '4px 0', borderBottom: 'var(--hairline)' }}>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                        letterSpacing: '0.12em', textTransform: 'uppercase' }}>
            {g.label}
          </span>
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
            {g.items.map(it => (
              <span key={it} className="mono" style={{ fontSize: 11,
                            padding: '4px 8px', background: 'var(--paper)',
                            border: 'var(--hairline)', borderRadius: 3,
                            color: 'var(--ink-2)' }}>
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
    <div style={{ display: 'grid', gridTemplateColumns: '280px 1fr',
                   gap: 0, height: '100%',
                   background: 'var(--paper)' }}>
      {/* Left rail — sticky summary */}
      <aside style={{ borderRight: 'var(--hairline)',
                       padding: '32px 24px 24px',
                       background: 'var(--paper-2)',
                       display: 'flex', flexDirection: 'column', gap: 16 }}>
        <button title="Change icon"
                style={{ width: 80, height: 80, borderRadius: 12,
                          background: icon.bg, color: icon.fg,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          border: '1px solid var(--edge)',
                          alignSelf: 'flex-start', position: 'relative' }}>
          <span className="kanji" style={{ fontSize: 40 }}>{icon.value}</span>
          <span style={{ position: 'absolute', bottom: -7, right: -7,
                          width: 22, height: 22, borderRadius: '50%',
                          background: 'var(--paper)', border: 'var(--hairline)',
                          fontSize: 11, color: 'var(--ink-3)',
                          display: 'flex', alignItems: 'center', justifyContent: 'center' }}>✎</span>
        </button>
        <div>
          <div className="display" style={{ fontSize: 22, fontWeight: 400,
                        letterSpacing: '-0.01em', lineHeight: 1.15 }}>
            {project.name}
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-3)', marginTop: 4 }}>
            {project.client}
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-2)', marginTop: 8,
                         lineHeight: 1.5, fontStyle: 'italic' }}>
            {project.goal}
          </div>
        </div>

        <div style={{ height: 1, background: 'var(--edge)' }}/>

        {/* quick facts */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <QuickFact label="repos"   value={project.repos.length}/>
          <QuickFact label="skills"  value={S.skills.filter(s=>s.on).length + " of " + S.skills.length}/>
          <QuickFact label="links"   value={S.links.length}/>
          <QuickFact label="backlog" value={S.backlog.length}/>
        </div>

        <div style={{ height: 1, background: 'var(--edge)' }}/>

        {/* anchor nav */}
        <nav style={{ display: 'flex', flexDirection: 'column', gap: 4, marginTop: 'auto' }}>
          {sections.map(id => (
            <button key={id} onClick={() => setActive(id)}
                    style={{ textAlign: 'left', padding: '8px 8px',
                              fontSize: 13, color: active===id ? 'var(--ink)' : 'var(--ink-3)',
                              background: active===id ? 'var(--paper)' : 'transparent',
                              borderRadius: 4,
                              borderLeft: active===id ? '2px solid var(--accent)' : '2px solid transparent',
                              fontWeight: active===id ? 500 : 400,
                              textTransform: 'capitalize' }}>
              {id}
            </button>
          ))}
        </nav>
      </aside>

      {/* Right — document */}
      <div style={{ padding: '32px 48px', overflow: 'auto', maxHeight: '100%' }}>
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
              <div key={label} style={{ display: 'grid', gridTemplateColumns: '120px 1fr auto',
                                          gap: 12, alignItems: 'center',
                                          padding: '8px 0', borderBottom: 'var(--hairline)' }}>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                              letterSpacing: '0.12em', textTransform: 'uppercase' }}>{label}</span>
                <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                  {items.length > 0 ? items.map(it => (
                    <span key={it} className="mono" style={{ fontSize: 11,
                                  padding: '4px 8px', background: 'var(--paper-2)',
                                  border: 'var(--hairline)', borderRadius: 3,
                                  color: 'var(--ink-2)' }}>{it}</span>
                  )) : <span style={{ fontSize: 11, color: 'var(--ink-4)', fontStyle: 'italic' }}>none</span>}
                </div>
                <button style={{ fontSize: 11, color: 'var(--accent)' }}>+ add</button>
              </div>
            ))}
        </V2Block>

        <V2Block id="repos" title="Repos" desc="Folders sensei watches for this project.">
          {project.repos.map(r => (
            <div key={r.id} style={{ display: 'grid', gridTemplateColumns: '1fr auto auto',
                                      gap: 12, alignItems: 'baseline',
                                      padding: '12px 0', borderBottom: 'var(--hairline)' }}>
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{r.id}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
                  {r.path}
                </div>
              </div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {r.size} · {r.lang}
              </span>
              <button style={{ fontSize: 11, color: 'var(--ink-3)' }}>remove</button>
            </div>
          ))}
          <button style={{ marginTop: 8, fontSize: 13, color: 'var(--accent)' }}>+ add repo</button>
        </V2Block>

        <V2Block id="links" title="Links" desc="Docs, dashboards, runbooks — anything sensei should remember.">
          {S.links.map(l => (
            <div key={l.id} style={{ display: 'grid', gridTemplateColumns: '90px 1fr auto',
                                      gap: 12, alignItems: 'baseline',
                                      padding: '12px 0', borderBottom: 'var(--hairline)' }}>
              <span className="mono" style={{ fontSize: 11, color: 'var(--accent)',
                            letterSpacing: '0.1em' }}>{l.kind}</span>
              <div>
                <div style={{ fontSize: 13 }}>{l.label}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>{l.url}</div>
              </div>
              <button style={{ fontSize: 11, color: 'var(--ink-3)' }}>edit</button>
            </div>
          ))}
          <button style={{ marginTop: 8, fontSize: 13, color: 'var(--accent)' }}>+ add link</button>
        </V2Block>

        <V2Block id="guidelines" title="Guidelines"
                 desc="Rules assistants should follow when working on this project.">
          {S.guidelines.map(g => (
            <div key={g.id} style={{ padding: '12px 0', borderBottom: 'var(--hairline)',
                                      fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }}>
              {g.rule}
            </div>
          ))}
          <button style={{ marginTop: 8, fontSize: 13, color: 'var(--accent)' }}>+ add rule</button>
        </V2Block>

        <V2Block id="backlog" title="Backlog"
                 desc="Things sensei should surface when relevant.">
          {S.backlog.map(b => (
            <div key={b.id} style={{ display: 'grid', gridTemplateColumns: '12px 1fr auto',
                                      gap: 12, alignItems: 'baseline',
                                      padding: '12px 0', borderBottom: 'var(--hairline)' }}>
              <span style={{ width: 6, height: 6, borderRadius: '50%',
                              background: 'var(--ink-4)', marginTop: 4 }}/>
              <div style={{ fontSize: 13, color: 'var(--ink)' }}>{b.task}</div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{b.added}</span>
            </div>
          ))}
          <button style={{ marginTop: 8, fontSize: 13, color: 'var(--accent)' }}>+ add task</button>
        </V2Block>
      </div>
    </div>
  );
}

function V2Block({ id, title, desc, children }) {
  return (
    <section id={id}
             style={{ paddingBottom: 32, marginBottom: 32,
                       borderBottom: '1px solid var(--edge)' }}>
      <div style={{ marginBottom: 16 }}>
        <h2 className="display" style={{ fontSize: 22, fontWeight: 400,
                       letterSpacing: '-0.01em', margin: 0 }}>
          {title}
        </h2>
        {desc && (
          <p style={{ fontSize: 13, color: 'var(--ink-3)',
                       margin: '4px 0 0', maxWidth: 560, lineHeight: 1.5 }}>
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
    <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr auto',
                   gap: 12, alignItems: multiline ? 'flex-start' : 'baseline',
                   padding: '12px 0', borderBottom: 'var(--hairline)' }}>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                    letterSpacing: '0.12em', textTransform: 'uppercase' }}>{label}</span>
      <div style={{ fontSize: 13, color: 'var(--ink)',
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
    <div style={{ display: 'flex', justifyContent: 'space-between',
                   alignItems: 'baseline' }}>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                    letterSpacing: '0.1em', textTransform: 'uppercase' }}>{label}</span>
      <span className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>{value}</span>
    </div>
  );
}

function SettingsCard({ title, action, children }) {
  return (
    <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                  borderRadius: 8, padding: '16px 16px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                    marginBottom: 8 }}>
        <h3 className="display" style={{ fontSize: 13, fontWeight: 400, margin: 0,
                                         letterSpacing: '0.01em' }}>{title}</h3>
        {action && (
          <button style={{ fontSize: 11, color: 'var(--accent)' }}>{action}</button>
        )}
      </div>
      {children}
    </div>
  );
}
function Field({ label, value, multiline }) {
  return (
    <div style={{ padding: '8px 0', borderBottom: 'var(--hairline)' }}>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase' }}>{label}</div>
      <div style={{ fontSize: 13, marginTop: 4,
                     fontStyle: multiline ? 'italic' : 'normal',
                     color: multiline ? 'var(--ink-2)' : 'var(--ink)' }}>{value}</div>
    </div>
  );
}
function ToggleChip({ on }) {
  return (
    <span className="mono" style={{
      fontSize: 11, padding: '4px 8px', borderRadius: 3,
      background: on ? 'var(--success-soft)' : 'var(--paper-3)',
      color: on ? 'var(--success)' : 'var(--ink-3)'
    }}>{on ? "on" : "off"}</span>
  );
}
function ToggleRow({ label, on, value, tone }) {
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr auto', gap: 8,
                  alignItems: 'center', padding: '8px 0', borderBottom: 'var(--hairline)' }}>
      <span style={{ fontSize: 13, color: tone === "warn" ? 'var(--warning)' : 'var(--ink)' }}>
        {label}
      </span>
      {value ? (
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{value}</span>
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
    <div style={{
      position: 'absolute', inset: 0, background: 'oklch(0.22 0.012 50 / 0.4)',
      display: 'flex', justifyContent: 'flex-end', zIndex: 50
    }} onClick={onClose}>
      <div onClick={(e) => e.stopPropagation()}
           style={{ width: 520, background: 'var(--paper)',
                    boxShadow: '-12px 0 32px rgba(0,0,0,0.15)',
                    display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <div style={{ padding: '24px 24px 12px', borderBottom: 'var(--hairline)',
                      display: 'flex', alignItems: 'flex-start', gap: 12 }}>
          <span className="kanji" style={{ fontSize: 28, color: 'var(--accent)', lineHeight: 1 }}>
            {rec.kanji || "送"}
          </span>
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                           textTransform: 'uppercase', marginBottom: 4 }}>
              {mode === "send" ? "Send prompt" : "Customize prompt"}
            </div>
            <div className="display" style={{ fontSize: 17, fontWeight: 400, letterSpacing: '-0.005em' }}>
              {rec.promptTitle || rec.title}
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }} className="mono">
              cwd · {rec.cwd || "—"}
            </div>
          </div>
          <button onClick={onClose} style={{ fontSize: 13, color: 'var(--ink-3)' }}>✕</button>
        </div>

        {/* ACP picker */}
        <div style={{ padding: '12px 24px', borderBottom: 'var(--hairline)' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>Send to</div>
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
            {acps.map(a => {
              const on = acp === a.id;
              return (
                <button key={a.id} onClick={() => setAcp(a.id)}
                        style={{ padding: '8px 12px',
                                 borderRadius: 5, fontSize: 11, textAlign: 'left',
                                 background: on ? 'var(--ink)' : 'var(--paper-2)',
                                 color: on ? 'var(--paper)' : 'var(--ink)',
                                 border: on ? 'none' : 'var(--hairline)' }}>
                  <div style={{ fontWeight: 500 }}>{a.label}</div>
                  <div className="mono" style={{ fontSize: 11, opacity: 0.6 }}>{a.sub}</div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Prompt editor */}
        <div style={{ flex: 1, padding: '12px 24px', overflow: 'auto' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>
            {mode === "send" ? "Prompt · preview" : "Prompt · editable"}
          </div>
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            readOnly={mode === "send"}
            style={{
              width: '100%', minHeight: 300, padding: 12,
              fontFamily: 'var(--font-mono)', fontSize: 13, lineHeight: 1.6,
              background: 'var(--paper-2)', border: 'var(--hairline)',
              borderRadius: 6, color: 'var(--ink)', resize: 'vertical'
            }}/>

          {rec.evidence && (
            <div style={{ marginTop: 12, fontSize: 11, color: 'var(--ink-3)' }}>
              <div style={{ letterSpacing: '0.14em', textTransform: 'uppercase', marginBottom: 4 }}>
                Evidence attached
              </div>
              <div className="mono">{rec.evidence.join(" · ")}</div>
            </div>
          )}
        </div>

        {/* Footer actions */}
        <div style={{ padding: '12px 24px', borderTop: 'var(--hairline)',
                      display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            {text.length.toLocaleString()} chars · will launch in {acp}
          </span>
          <span style={{ flex: 1 }}/>
          <button onClick={onClose}
                  style={{ padding: '8px 12px', fontSize: 13,
                           color: 'var(--ink-2)', border: 'var(--ink-line)', borderRadius: 5 }}>
            cancel
          </button>
          <button style={{ padding: '8px 16px', fontSize: 13,
                           background: 'var(--ink)', color: 'var(--paper)', borderRadius: 5 }}>
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
