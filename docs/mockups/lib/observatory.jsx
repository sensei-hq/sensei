// Sensei Observatory — daily launch view.
// "Mature" state by default; "early" state via tweak.
// Single calm focal message (the hero koan) + secondary items.
// Left rail: Projects (active & recent) + Sections.

const { useState: oS, useEffect: oE, useMemo: oM } = React;

// ─── Fake daily data ─────────────────────────────────────────
window.OBS_DATA = {
  today: "Wed · 22 Apr",
  greetings: {
    morning: ["Good morning", "おはよう", "Settled in?"],
    afternoon: ["Good afternoon", "こんにちは"],
    evening: ["Evening", "こんばんは"]
  },

  // Signature metric — "First-Try-Right" rate, last 14 days
  ftr: {
    value: 0.78,
    delta: +0.06,           // vs prior 14d
    prev: 0.72,
    trend14: [0.71, 0.69, 0.74, 0.72, 0.68, 0.70, 0.73,
              0.75, 0.72, 0.78, 0.74, 0.79, 0.76, 0.78]
  },

  // The hero teaching — always exactly one, the most important thing right now.
  hero: {
    mature: {
      kanji: "聴",
      koan: "The AI does not know your auth.",
      body: "Three sessions corrected this week in lumen-auth — all touched refresh or device flow. There is no integration-test persona for this module yet.",
      impact: "Projected FTR + 14% in Lumen Cloud",
      action: "Draft a persona",
      source: "from s-2891 · s-2889 · s-2886",
      noticed: "noticed 2 days ago"
    },
    early: {
      kanji: "観",
      koan: "Still listening.",
      body: "Sensei has watched 4 sessions so far. A few early signals are forming in lumen-auth, but nothing confident enough to teach yet.",
      impact: "~2–3 more sessions until first lesson",
      action: null,
      source: "s-2891 · s-2890 · s-2889 · s-2888",
      noticed: "since setup · 2d ago"
    }
  },

  // Insights behind the hero — things worth seeing, never more than 3
  insights: {
    mature: [
      { kanji: "繰", label: "Pattern recurring",
        text: "Cache invalidation missed again in session s-2891.",
        tag: "3rd time", tone: "warn" },
      { kanji: "昇", label: "Teaching adopted",
        text: "Canvas smoothing pattern promoted to rule — applied in 4 subsequent sessions.",
        tag: "+7% FTR", tone: "good" },
      { kanji: "探", label: "Drift detected",
        text: "brand-tokens README is 47 days old. 3 APIs have drifted.",
        tag: "low urgency", tone: "mute" }
    ],
    early: [
      { kanji: "耳", label: "Listening",
        text: "Watching prompt style in lumen-canvas. Early signal: you prefer terse instructions.",
        tag: "forming", tone: "mute" },
      { kanji: "試", label: "Calibrating",
        text: "Sensei is still learning your correction cadence. Too early to suggest rules.",
        tag: "—", tone: "mute" }
    ]
  },

  // Rules/skills sensei has actually adopted (learning → system behavior)
  adopted: {
    mature: [
      { when: "2d ago", what: "Canvas smoothing pattern → rule",
        scope: "lumen-studio", source: "from session s-2890" },
      { when: "5d ago", what: "Auth refresh · clock-skew tolerance",
        scope: "lumen-cloud", source: "from session s-2891" },
      { when: "1w ago", what: "Token drift watchdog enabled",
        scope: "brand-kit", source: "manual + sensei recommend" }
    ],
    early: []
  },

  // Recent sessions — tight list
  recentSessions: [
    { id: "s-2891", project: "lumen-auth",   title: "Fix refresh token rotation",
      time: "10:42", duration: "38m", ftr: false, corrections: 3 },
    { id: "s-2890", project: "lumen-canvas", title: "Bezier smoothing tool",
      time: "09:15", duration: "22m", ftr: true,  corrections: 0 },
    { id: "s-2889", project: "lumen-auth",   title: "OAuth device flow",
      time: "Yesterday", duration: "1h 12m", ftr: false, corrections: 4 },
    { id: "s-2888", project: "brand-tokens", title: "Dark-mode color ramps",
      time: "Yesterday", duration: "18m", ftr: true,  corrections: 0 }
  ],

  projects: {
    active: [
      { id: "lumen-studio", kanji: "工", name: "Lumen Studio", ftr: 0.82, sessions7d: 41, warn: false },
      { id: "lumen-cloud",  kanji: "雲", name: "Lumen Cloud",  ftr: 0.64, sessions7d: 28, warn: true  },
      { id: "brand-kit",    kanji: "紋", name: "Brand Kit",    ftr: 0.91, sessions7d: 12, warn: false }
    ],
    recent: [
      { id: "sketch-tool",  kanji: "筆", name: "Sketch tool",   lastSeen: "3w ago",  sessions: 6 },
      { id: "old-docs",     kanji: "巻", name: "Docs site",     lastSeen: "2mo ago", sessions: 14 }
    ],
    archived: [
      { id: "prototype-x",  kanji: "試", name: "Prototype X",   archived: "6mo ago" }
    ]
  }
};

// ─── Small helpers ───────────────────────────────────────────
function ObsSparkline({ data, width = 120, height = 30, color = 'var(--shu)' }) {
  const min = Math.min(...data), max = Math.max(...data);
  const range = max - min || 1;
  const pad = 2;
  const step = (width - pad*2) / (data.length - 1);
  const pts = data.map((v,i) => [
    pad + i*step,
    pad + (height - pad*2) * (1 - (v - min)/range)
  ]);
  const d = pts.map((p,i) => (i ? "L" : "M") + p[0].toFixed(1) + " " + p[1].toFixed(1)).join(" ");
  return (
    <svg width={width} height={height} style={{ color, display: 'block', overflow: 'visible' }}>
      <path d={d} className="sparkline-path"/>
      <circle cx={pts[pts.length-1][0]} cy={pts[pts.length-1][1]} r={2.5} fill="currentColor"/>
    </svg>
  );
}

function ObsFtrRing({ value, delta, size = 110 }) {
  const r = (size - 10) / 2;
  const cx = size/2, cy = size/2;
  const start = -140, sweep = 300;
  const toXY = (deg) => {
    const rad = (deg * Math.PI) / 180;
    return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)];
  };
  const arcPath = (a0, a1) => {
    const [x0, y0] = toXY(a0);
    const [x1, y1] = toXY(a1);
    const large = a1 - a0 > 180 ? 1 : 0;
    return `M ${x0.toFixed(2)} ${y0.toFixed(2)} A ${r} ${r} 0 ${large} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
  };
  const endAngle = start + sweep * Math.max(0.02, Math.min(0.99, value));
  return (
    <svg width={size} height={size} style={{ display: 'block' }}>
      <path d={arcPath(start, start + sweep)}
            stroke="currentColor" strokeOpacity="0.1"
            strokeWidth="8" strokeLinecap="round" fill="none"/>
      <path d={arcPath(start, endAngle)}
            stroke="var(--shu)" strokeWidth="8" strokeLinecap="round" fill="none"/>
      <circle cx={toXY(start)[0]} cy={toXY(start)[1]} r="4.5" fill="var(--shu)"/>
      <text x={cx} y={cy - 4} textAnchor="middle" dominantBaseline="central"
             fontFamily="var(--font-display)" fontSize={size * 0.28} fontWeight="400">
        {Math.round(value * 100)}
      </text>
      <text x={cx} y={cy + size*0.18} textAnchor="middle" dominantBaseline="central"
             fontFamily="var(--font-ui)" fontSize={size * 0.09}
             letterSpacing="0.14em" fill="var(--sumi-3)">
        FTR · 14d
      </text>
    </svg>
  );
}

// ─── Observatory (daily) ─────────────────────────────────────
function ObservatoryDaily({ stateMode = "mature", firstEntry = false, onBack }) {
  const [mode, setMode] = oS(stateMode);       // "mature" | "early"
  const [section, setSection] = oS("home");    // "home" | "projects" | "project" | "sessions" | "patterns" | "libraries" | "registry" | "teachings" | "settings"
  const [activeProjectId, setActiveProjectId] = oS(null);
  const [toast, setToast] = oS(firstEntry ? "welcome" : null);
  const D = window.OBS_DATA;

  const openProject = (id) => { setActiveProjectId(id); setSection("project"); };

  oE(() => { setMode(stateMode); }, [stateMode]);
  oE(() => {
    if (toast !== "welcome") return;
    const t = setTimeout(() => setToast(null), 5200);
    return () => clearTimeout(t);
  }, [toast]);

  const hero = D.hero[mode];
  const insights = D.insights[mode];
  const adopted = D.adopted[mode];

  return (
    <div className="sensei" data-screen-label="Observatory · Daily"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title={`Sensei  先生  ·  observatory`}/>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '240px 1fr', minHeight: 0 }}>
        <ObsSidebar section={section} setSection={setSection}
                    activeProjectId={activeProjectId}
                    onOpenProject={openProject}
                    mode={mode} setMode={setMode}/>
        <main style={{ overflow: 'auto', position: 'relative' }}>
          {section === "home"      && <ObsHome mode={mode} hero={hero} insights={insights} adopted={adopted} D={D} onOpenProject={openProject}/>}
          {section === "projects"  && <ProjectsIndexA embedded={true} onOpenProject={openProject}/>}
          {section === "project"   && <ProjectPageTopTabs embedded={true} projectId={activeProjectId}
                                                          onBack={() => setSection("projects")}/>}
          {section === "libraries" && <LibrariesVariantA/>}
          {section === "learnings" && <LearningsPage/>}
          {section === "instruments-playground" && <InstrumentsPlaygroundSimple/>}
          {section === "instruments-replay"     && <InstrumentsReplaySimple/>}
          {section === "instruments-insights"   && <InstrumentsInsightsSimple/>}
          {section !== "home" && section !== "projects" && section !== "project" &&
           section !== "libraries" &&
           section !== "learnings" &&
           section !== "instruments-playground" &&
           section !== "instruments-replay" &&
           section !== "instruments-insights" &&
            <ObsPlaceholder section={section} onBack={() => setSection("home")}/>}

          {toast === "welcome" && (
            <FirstEntryToast onDismiss={() => setToast(null)} mode={mode}/>
          )}
        </main>
      </div>
    </div>
  );
}

// ─── Sidebar ────────────────────────────────────────────────
function ObsSidebar({ section, setSection, activeProjectId, onOpenProject, mode, setMode }) {
  const D = window.OBS_DATA;
  const NavItem = ({ id, kanji, label, badge }) => (
    <button onClick={() => setSection(id)}
            style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 10,
              alignItems: 'center', width: '100%',
              padding: '7px 10px', borderRadius: 6, textAlign: 'left',
              background: section === id ? 'var(--paper-3)' : 'transparent',
              color: section === id ? 'var(--sumi)' : 'var(--sumi-2)',
              fontSize: 13
            }}>
      <span className="kanji" style={{ fontSize: 13, width: 14,
                    color: section === id ? 'var(--shu)' : 'var(--sumi-3)' }}>{kanji}</span>
      <span>{label}</span>
      {badge != null && (
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>{badge}</span>
      )}
    </button>
  );

  return (
    <aside style={{ borderRight: 'var(--hairline)', padding: '22px 14px',
                     background: 'var(--paper-2)',
                     display: 'flex', flexDirection: 'column', gap: 20,
                     overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, padding: '0 6px' }}>
        <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>先</span>
        <span className="display" style={{ fontSize: 17 }}>Sensei</span>
      </div>

      <div>
        <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', padding: '0 10px 8px' }}>Observatory</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          <NavItem id="home"      kanji="家" label="Today"/>
          <NavItem id="projects"  kanji="場" label="Projects"   badge={D.projects.active.length + D.projects.recent.length}/>
          <NavItem id="sessions"  kanji="録" label="Sessions"   badge="41"/>
          <NavItem id="learnings" kanji="学" label="Learnings"  badge="6"/>
          <NavItem id="libraries" kanji="庫" label="Libraries"  badge="14"/>
          <div style={{ padding: '2px 10px' }}>
            <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 10,
                           alignItems: 'center', padding: '7px 0',
                           color: 'var(--sumi-2)', fontSize: 13 }}>
              <span className="kanji" style={{ fontSize: 13, width: 14,
                            color: 'var(--sumi-3)' }}>具</span>
              <span>Instruments</span>
              <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>7</span>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 1,
                           paddingLeft: 24 }}>
              <NavItem id="instruments-playground" kanji="試" label="Playground"/>
              <NavItem id="instruments-replay"     kanji="録" label="Replay"/>
              <NavItem id="instruments-insights"   kanji="照" label="Insights"/>
            </div>
          </div>
          <NavItem id="settings"  kanji="設" label="Settings"/>
        </div>
      </div>

      <div>
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                       padding: '0 10px 8px' }}>
          <span style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                          textTransform: 'uppercase' }}>Active projects</span>
          <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
            {D.projects.active.length}
          </span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          {D.projects.active.map(p => {
            const on = section === "project" && activeProjectId === p.id;
            return (
              <button key={p.id} onClick={() => onOpenProject && onOpenProject(p.id)}
                      style={{
                display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 10,
                alignItems: 'center', width: '100%',
                padding: '8px 10px', borderRadius: 6, textAlign: 'left',
                background: on ? 'var(--paper-3)' : 'transparent',
                color: on ? 'var(--sumi)' : 'var(--sumi-2)', fontSize: 12.5,
                cursor: 'pointer'
              }}>
                <span className="kanji" style={{ fontSize: 13, width: 14,
                            color: p.warn ? 'var(--amber)' : 'var(--shu)' }}>{p.kanji}</span>
                <span>{p.name}</span>
                <span className="mono" style={{ fontSize: 10,
                              color: p.warn ? 'var(--amber)' : 'var(--sumi-3)' }}>
                  {Math.round(p.ftr * 100)}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      <div>
        <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', padding: '0 10px 8px' }}>Dormant</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          {D.projects.recent.map(p => {
            const on = section === "project" && activeProjectId === p.id;
            return (
              <button key={p.id} onClick={() => onOpenProject && onOpenProject(p.id)}
                      style={{
                display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 10,
                alignItems: 'center', width: '100%',
                padding: '7px 10px', borderRadius: 6, textAlign: 'left',
                background: on ? 'var(--paper-3)' : 'transparent',
                color: on ? 'var(--sumi-2)' : 'var(--sumi-3)', fontSize: 12.5,
                opacity: on ? 1 : 0.82, cursor: 'pointer'
              }}>
                <span className="kanji" style={{ fontSize: 12, width: 14, opacity: 0.6 }}>{p.kanji}</span>
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {p.name}
                </span>
                <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
                  {p.lastSeen.split(' ')[0]}
                </span>
              </button>
            );
          })}
          <button style={{
            padding: '6px 10px', fontSize: 11, color: 'var(--sumi-4)', textAlign: 'left'
          }}>
            + {D.projects.archived.length} archived
          </button>
        </div>
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ padding: '10px 10px 0', borderTop: 'var(--hairline)',
                     fontSize: 10, color: 'var(--sumi-3)', lineHeight: 1.6 }}>
        <span className="mono">daemon · running</span><br/>
        <span style={{ color: 'var(--sumi-4)' }}>last heartbeat 2s ago</span>
      </div>
    </aside>
  );
}

// ─── Home (today) ───────────────────────────────────────────
function ObsHome({ mode, hero, insights, adopted, D }) {
  return (
    <div style={{ padding: '36px 48px 48px', maxWidth: 1060, margin: '0 auto' }}>
      {/* Greeting strip */}
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     marginBottom: 32 }}>
        <div>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 6 }}>
            {D.today}
          </div>
          <h1 className="display" style={{ fontSize: 28, fontWeight: 400, margin: 0,
                        letterSpacing: '-0.01em' }}>
            Good morning, Aiko.
          </h1>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16,
                       color: mode === "early" ? 'var(--sumi-3)' : 'var(--sumi-2)' }}>
          <ObsFtrRing value={D.ftr.value} delta={D.ftr.delta} size={86}/>
          <div style={{ minWidth: 150 }}>
            <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginBottom: 4 }}>
              First-Try-Right · 14 days
            </div>
            <ObsSparkline data={D.ftr.trend14} width={150} height={36}
                           color={mode === "early" ? 'var(--sumi-3)' : 'var(--shu)'}/>
            <div style={{ fontSize: 11, color: D.ftr.delta >= 0 ? 'var(--jade)' : 'var(--amber)',
                           marginTop: 3 }} className="mono">
              {D.ftr.delta >= 0 ? "↑" : "↓"} {Math.abs(Math.round(D.ftr.delta * 100))}% vs prior
            </div>
          </div>
        </div>
      </div>

      {/* Hero koan — the one focal thing */}
      <ObsHero hero={hero} mode={mode}/>

      {/* Two columns: Insights + Adopted teachings */}
      <div style={{ marginTop: 36, display: 'grid', gridTemplateColumns: '1.4fr 1fr',
                     gap: 30 }}>
        <ObsInsights items={insights} mode={mode}/>
        <ObsAdopted items={adopted} mode={mode}/>
      </div>

      {/* Recent sessions — compact */}
      <ObsRecentSessions sessions={D.recentSessions}/>
    </div>
  );
}

function ObsHero({ hero, mode }) {
  return (
    <div style={{
      position: 'relative',
      padding: '32px 34px 30px',
      background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 12,
      display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 28
    }}>
      {/* Giant kanji — still the focal anchor */}
      <div style={{ position: 'relative' }}>
        <div className="kanji" style={{
          fontSize: 96, color: 'var(--shu)', lineHeight: 1,
          opacity: mode === "early" ? 0.55 : 1
        }}>{hero.kanji}</div>
        <div style={{
          position: 'absolute', left: -6, top: -4,
          fontSize: 9, letterSpacing: '0.18em', color: 'var(--sumi-3)',
          textTransform: 'uppercase', writingMode: 'vertical-rl',
          transform: 'rotate(180deg)', height: 96
        }}>
          {mode === "early" ? "sensei is listening" : "sensei speaks"}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }}>
        <div className="display" style={{
          fontSize: 28, fontWeight: 400, letterSpacing: '-0.01em',
          lineHeight: 1.2, marginBottom: 14, color: 'var(--sumi)'
        }}>
          {hero.koan}
        </div>
        <p style={{ fontSize: 14.5, color: 'var(--sumi-2)', lineHeight: 1.65,
                     margin: '0 0 18px', maxWidth: 620 }}>
          {hero.body}
        </p>

        <div style={{ display: 'flex', alignItems: 'center', gap: 16, flexWrap: 'wrap',
                       marginTop: 'auto' }}>
          {hero.action && (
            <button style={{
              padding: '9px 18px', fontSize: 13, background: 'var(--sumi)',
              color: 'var(--paper)', borderRadius: 6, letterSpacing: 0.2,
              display: 'inline-flex', alignItems: 'center', gap: 8
            }}>
              {hero.action} →
            </button>
          )}
          <div style={{ fontSize: 12, color: 'var(--shu)',
                         display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ width: 5, height: 5, borderRadius: '50%', background: 'var(--shu)' }}/>
            {hero.impact}
          </div>
          <span style={{ flex: 1 }}/>
          <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
            {hero.source} · {hero.noticed}
          </span>
        </div>
      </div>
    </div>
  );
}

function ObsInsights({ items, mode }) {
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     marginBottom: 16 }}>
        <h2 className="display" style={{ fontSize: 17, fontWeight: 400, margin: 0,
                      letterSpacing: '-0.005em' }}>
          {mode === "early" ? "Early signals" : "Also worth noticing"}
        </h2>
        <button style={{ fontSize: 11, color: 'var(--sumi-3)' }}>all insights →</button>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {items.map((x, i) => {
          const toneColor =
            x.tone === "warn" ? 'var(--amber)' :
            x.tone === "good" ? 'var(--jade)'  : 'var(--sumi-3)';
          return (
            <button key={i} style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 14,
              alignItems: 'start', padding: '14px 16px', textAlign: 'left',
              borderRadius: 6, borderBottom: 'var(--hairline)'
            }}>
              <span className="kanji" style={{ fontSize: 19, color: toneColor,
                            width: 26 }}>{x.kanji}</span>
              <div>
                <div style={{ fontSize: 10.5, letterSpacing: '0.14em',
                               textTransform: 'uppercase', color: 'var(--sumi-3)',
                               marginBottom: 4 }}>
                  {x.label}
                </div>
                <div style={{ fontSize: 13.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
                  {x.text}
                </div>
              </div>
              <span className="mono" style={{ fontSize: 10.5, color: toneColor,
                            padding: '3px 8px', borderRadius: 3,
                            background: x.tone === "warn" ? 'var(--amber-soft)' :
                                         x.tone === "good" ? 'var(--jade-soft)' :
                                         'var(--paper-3)',
                            whiteSpace: 'nowrap' }}>
                {x.tag}
              </span>
            </button>
          );
        })}
        {items.length === 0 && (
          <div style={{ padding: 20, fontSize: 13, color: 'var(--sumi-4)', fontStyle: 'italic',
                         textAlign: 'center',
                         border: '1px dashed var(--paper-edge)', borderRadius: 8 }}>
            Nothing yet. Keep working — sensei watches.
          </div>
        )}
      </div>
    </div>
  );
}

function ObsAdopted({ items, mode }) {
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     marginBottom: 16 }}>
        <h2 className="display" style={{ fontSize: 17, fontWeight: 400, margin: 0,
                      letterSpacing: '-0.005em' }}>
          System has learned
        </h2>
        <button style={{ fontSize: 11, color: 'var(--sumi-3)' }}>all teachings →</button>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {items.map((x, i) => (
          <div key={i} style={{
            padding: '12px 14px', borderRadius: 6,
            background: 'var(--paper-2)', border: 'var(--hairline)',
            borderLeft: '2px solid var(--shu)'
          }}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 4 }}>
              <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
                {x.when}
              </span>
              <span style={{ fontSize: 10, color: 'var(--sumi-4)' }}>·</span>
              <span className="mono" style={{ fontSize: 10, color: 'var(--shu)' }}>
                {x.scope}
              </span>
            </div>
            <div style={{ fontSize: 13, color: 'var(--sumi)', lineHeight: 1.45 }}>
              {x.what}
            </div>
            <div style={{ fontSize: 10.5, color: 'var(--sumi-4)', marginTop: 4 }}>
              {x.source}
            </div>
          </div>
        ))}
        {items.length === 0 && (
          <div style={{ padding: '24px 18px',
                         border: '1px dashed var(--paper-edge)', borderRadius: 8,
                         textAlign: 'center' }}>
            <div className="kanji" style={{ fontSize: 28, color: 'var(--sumi-4)',
                          marginBottom: 8 }}>空</div>
            <div style={{ fontSize: 12, color: 'var(--sumi-3)', lineHeight: 1.5 }}>
              No teachings adopted yet.<br/>
              Sensei needs a few more sessions to be confident.
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function ObsRecentSessions({ sessions }) {
  return (
    <div style={{ marginTop: 40 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     marginBottom: 14 }}>
        <h2 className="display" style={{ fontSize: 17, fontWeight: 400, margin: 0 }}>
          Recent sessions
        </h2>
        <button style={{ fontSize: 11, color: 'var(--sumi-3)' }}>all sessions →</button>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {sessions.map(s => (
          <button key={s.id} style={{
            display: 'grid',
            gridTemplateColumns: 'auto 120px 1fr auto auto auto',
            gap: 16, alignItems: 'center',
            padding: '12px 4px', textAlign: 'left',
            borderBottom: 'var(--hairline)'
          }}>
            <span style={{ width: 8, height: 8, borderRadius: '50%',
                            background: s.ftr ? 'var(--jade)' : 'var(--amber)' }}/>
            <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi-3)' }}>
              {s.project}
            </span>
            <span style={{ fontSize: 13, color: 'var(--sumi-2)',
                            overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {s.title}
            </span>
            <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
              {s.corrections === 0 ? "first-try" : `${s.corrections}×`}
            </span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)',
                          minWidth: 50, textAlign: 'right' }}>
              {s.duration}
            </span>
            <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-4)' }}>
              {s.time}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

function ObsPlaceholder({ section, onBack }) {
  const labels = {
    sessions:  { k: "録", t: "Sessions",  s: "every session sensei has witnessed" },
    patterns:  { k: "紋", t: "Patterns",  s: "recurring shapes across your work"  },
    libraries: { k: "庫", t: "Libraries", s: "small & internal libs sensei wraps with its own tools" },
    registry:  { k: "具", t: "Instruments", s: "available & installed MCPs · recommended by stack" },
    teachings: { k: "師", t: "Teachings", s: "what sensei has learned & applied"  },
    settings:  { k: "設", t: "Settings",  s: "folders · integrations · daemon"    }
  };
  const L = labels[section];
  return (
    <div style={{ padding: 60, textAlign: 'center', maxWidth: 520, margin: '0 auto' }}>
      <div className="kanji" style={{ fontSize: 56, color: 'var(--shu)', opacity: 0.5,
                                       marginBottom: 14 }}>{L.k}</div>
      <h1 className="display" style={{ fontSize: 32, fontWeight: 300, margin: '0 0 8px' }}>
        {L.t}
      </h1>
      <p style={{ fontSize: 13, color: 'var(--sumi-3)', margin: '0 0 24px' }}>{L.s}</p>
      <button onClick={onBack}
              style={{ fontSize: 12, padding: '8px 16px', border: 'var(--ink-line)',
                        borderRadius: 5, color: 'var(--sumi-2)' }}>
        ← Today
      </button>
    </div>
  );
}

function FirstEntryToast({ onDismiss, mode }) {
  return (
    <div style={{
      position: 'absolute', top: 24, left: '50%', transform: 'translateX(-50%)',
      background: 'var(--sumi)', color: 'var(--paper)',
      padding: '14px 22px 14px 18px', borderRadius: 10,
      display: 'flex', alignItems: 'center', gap: 16,
      boxShadow: '0 6px 24px rgba(0,0,0,0.2)', zIndex: 20,
      animation: 'toast-in .45s ease-out'
    }}>
      <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>礼</span>
      <div>
        <div className="display" style={{ fontSize: 14, fontWeight: 400, marginBottom: 2 }}>
          The observatory is open.
        </div>
        <div style={{ fontSize: 11.5, color: 'var(--paper-edge)', opacity: 0.75 }}>
          {mode === "early"
            ? "Come back after a few sessions. Sensei is still listening."
            : "Three projects · 81 sessions watched · 3 teachings adopted so far."}
        </div>
      </div>
      <button onClick={onDismiss} style={{ fontSize: 11, color: 'var(--sumi-4)',
                padding: '4px 8px', marginLeft: 8 }}>dismiss</button>
      <style>{`@keyframes toast-in {
        from { opacity: 0; transform: translate(-50%, -12px) }
        to { opacity: 1; transform: translateX(-50%) }
      }`}</style>
    </div>
  );
}

// ─── Harness for design canvas ───────────────────────────────
function ObservatoryDailyApp()       { return <ObservatoryDaily stateMode="mature"/>; }
function ObservatoryEarlyApp()       { return <ObservatoryDaily stateMode="early"/>;  }
function ObservatoryEntryApp()       { return <ObservatoryDaily stateMode="mature" firstEntry={true}/>; }

Object.assign(window, {
  ObservatoryDaily, ObservatoryDailyApp, ObservatoryEarlyApp, ObservatoryEntryApp
});
