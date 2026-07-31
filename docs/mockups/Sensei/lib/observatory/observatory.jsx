// Sensei Observatory — daily launch view.
// "Mature" state by default; "early" state via tweak.
// Single calm focal message (the hero koan) + secondary items.
// Left rail: Projects (active & recent) + Sections.

const { useState: oS, useEffect: oE, useMemo: oM } = React;

// ─── Daily data + Today screen → lib/observatory-today.jsx ───
// OBS_DATA and the Today screen (ObsHome/ObsHero/ObsInsights/ObsAdopted/
// ObsRecentSessions + ObsSparkline/ObsFtrStrip) now live in the shared module
// observatory-today.jsx so the website renders the same Today. Loaded first;
// referenced here as globals.

// ─── Observatory (daily) ─────────────────────────────────────
// Sub-section tabs — flattens the old nested sidebar groups (Memories,
// Impact, Instruments) into single rail items, surfacing their sibling
// views as an in-content tab strip instead of a collapsible nest.
const SUBNAV = {
  memories:    { items: [["memories","Anatomy"],["share-review","Sharing","4"],["consolidation","Consolidate","3"]] },
  impact:      { items: [["impact","Reports","3"],["impact-alert","Regressions","1","danger"]] },
  instruments: { items: [["instruments-playground","Playground"],["instruments-replay","Replay"],["instruments-health","Health"]] },
  dojo:        { items: [["dojo-connections","Connections"],["dojo-sharing","Sharing"]] },
};
function groupKeyOf(section) {
  for (const k in SUBNAV) if (SUBNAV[k].items.some(it => it[0] === section)) return k;
  return null;
}
function ObsSubTabs({ group, section, setSection }) {
  return (
    <div className="flex items-center border-b bg-paper shrink-0" style={{ gap: 6, padding: '12px 24px' }}>
      {group.items.map(([id, label, badge, tone]) => {
        const on = section === id;
        return (
          <button className="inline-flex items-center border-0 cursor-pointer" key={id} onClick={() => setSection(id)}
 style={{ gap: 6, fontSize: 12,
 padding: '8px 12px', borderRadius: 6,
 background: on ? 'var(--ink)' : 'var(--paper-3)',
 color: on ? 'var(--paper)' : 'var(--ink-2)', fontFamily: 'inherit' }}>
            {label}
            {badge != null && (
              <span className="mono font-semibold" style={{ fontSize: 10,
 color: tone === 'danger' ? (on ? 'var(--paper)' : 'var(--danger)')
 : (on ? 'var(--paper)' : 'var(--ink-3)'),
 background: (tone === 'danger' && !on) ? 'var(--danger-soft)' : 'transparent',
 borderRadius: 10, padding: tone === 'danger' ? '0 5px' : '0' }}>{badge}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}

function ObservatoryDaily({ stateMode = "mature", firstEntry = false, initialSection = null, onBack }) {
  const [mode, setMode] = oS(stateMode);       // "mature" | "early"
  // Sections actually routed below: home · projects · project · sessions · logs
  // · libraries · insights · memories · upgrades · share-review · consolidation
  // · traceability · impact · impact-alert · collective · instruments-* · configure
  // Landing section depends on maturity: until insights exist we open on
  // Projects (the real payoff of the scan), not an empty Today. Once sensei
  // is "mature" (insights formed) Today becomes the home view.
  const [section, setSection] = oS(initialSection || (stateMode === "early" ? "projects" : "home"));
  const [activeProjectId, setActiveProjectId] = oS(null);
  // Where the user opened the current project FROM, so "back" returns there
  // (Today / Projects index / wherever) instead of always dumping on Projects.
  const [projectOrigin, setProjectOrigin] = oS("projects");
  const [toast, setToast] = oS(() => {
    if (!firstEntry) return null;
    // Honor the "Don't show again" preference from Settings.
    try {
      const raw = localStorage.getItem("sensei-settings");
      if (raw && JSON.parse(raw).showWelcome === false) return null;
    } catch {}
    return "welcome";
  });
  const D = window.OBS_DATA;

  const openProject = (id) => {
    setProjectOrigin(section === "project" ? projectOrigin : section);
    setActiveProjectId(id);
    setSection("project");
  };

  oE(() => { setMode(stateMode); }, [stateMode]);
  oE(() => {
    if (toast !== "welcome") return;
    const t = setTimeout(() => setToast(null), 5200);
    return () => clearTimeout(t);
  }, [toast]);

  const hero = D.hero[mode];
  const insights = D.insights[mode];
  const adopted = D.adopted[mode];

  // Preferences (was "Configure") is a redirect to the settings surface — the
  // former setup wizard, reframed: scan first, free navigation, no gate. It
  // brings its own Tauri chrome and full-bleed layout, so we short-circuit
  // the observatory shell entirely. On close we return to the maturity-
  // appropriate landing (Projects while early, Today once mature).
  if (section === "configure") {
    const home = mode === "early" ? "projects" : "home";
    return (
      <SetupWizard mode="preferences"
                   onExit={() => setSection(home)}
                   onDone={() => setSection(home)}/>
    );
  }

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Daily"
 >
      <TauriChrome title={`Sensei  先生  ·  observatory`}/>

      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '240px 1fr' }}>
        <ObsSidebar section={section} setSection={setSection}
                    activeProjectId={activeProjectId}
                    onOpenProject={openProject}
                    mode={mode} setMode={setMode}/>
        <main className="overflow-hidden relative flex flex-col" >
          {(() => {
            const grp = SUBNAV[groupKeyOf(section)];
            // Instruments keeps its sub-tabs BELOW each screen's header (passed
            // in as `subNav`), so it isn't rendered here at the top.
            if (!grp || groupKeyOf(section) === "instruments") return null;
            return <ObsSubTabs group={grp} section={section} setSection={setSection}/>;
          })()}
          <div className="flex-1 min-h-0 relative" >
          {section === "intake"    && <IntakeScreen/>}
          {section === "home"      && <ObsHome mode={mode} hero={hero} insights={insights} adopted={adopted} D={D} onOpenProject={openProject}/>}
          {section === "projects"  && (
            <div className="h-full flex flex-col min-h-0" >
              {mode === "early" && <FirstSessionGuide onOpenToday={() => setSection("home")}
                                                      onOpenProject={openProject}/>}
              <div className="flex-1 min-h-0" >
                <ProjectsIndexA embedded={true} onOpenProject={openProject}/>
              </div>
            </div>
          )}
          {section === "project"   && <ProjectPageSidebar embedded={true} projectId={activeProjectId}
                                                          onBack={() => setSection(projectOrigin)}
                                                          onSwitchProject={() => setSection("projects")}/>}
          {section === "sessions"  && <SessionsDigestZen/>}
          {section === "logs"      && <ObsLogs/>}
          {section === "libraries" && <LibrariesVariantA embedded={true}/>}
          {section === "insights" && <LearningsTriage/>}
          {section === "memories" && <LearningsAnatomyV2/>}
          {section === "upgrades" && <InappDownstream embedded={true}/>}
          {section === "share-review"  && <InappShare embedded={true}/>}
          {section === "consolidation" && <ObsConsolidation/>}
          {section === "traceability" && <ObsTraceability/>}
          {section === "impact"        && <ObsImpact/>}
          {section === "impact-alert"  && <ObsNegativeAlert/>}
          {section === "dojo-connections" && <InappConnection embedded={true}/>}
          {section === "dojo-sharing"     && <ObsCollectiveSettings/>}
          {(() => {
            const instrNav = <ObsSubTabs group={SUBNAV.instruments} section={section} setSection={setSection}/>;
            return <>
              {section === "instruments-playground" && <InstrumentsPlaygroundSimple subNav={instrNav}/>}
              {section === "instruments-replay"     && <InstrumentsReplaySimple subNav={instrNav}/>}
              {section === "instruments-health"     && <InstrumentsHealthSimple subNav={instrNav}/>}
            </>;
          })()}
          {section !== "intake" &&
           section !== "home" && section !== "projects" && section !== "project" &&
           section !== "sessions" &&
           section !== "logs" &&
           section !== "libraries" &&
           section !== "insights" && section !== "memories" &&
           section !== "upgrades" && section !== "share-review" && section !== "consolidation" &&
           section !== "traceability" && section !== "impact" && section !== "impact-alert" &&
           section !== "dojo-connections" &&
           section !== "dojo-sharing" &&
           section !== "instruments-playground" &&
           section !== "instruments-replay" &&
           section !== "instruments-health" &&
            <ObsPlaceholder section={section} onBack={() => setSection("home")}/>}
          </div>

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
  const [focus, setFocus] = oS(false);  // "Focus" collapses the rail to what needs a decision
  const Cluster = ({ label }) => (
    <div style={{ fontSize: 9.5, letterSpacing: '0.14em' }} className="pt-3 pb-1 px-2 uppercase text-ink-4 font-semibold" >{label}</div>
  );
  const NavItem = ({ id, kanji, label, badge, badgeTone, alert, match }) => {
    const active = section === id || (match && match.includes(section));
    return (
    <button onClick={() => setSection(id)}
 style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 6,
 background: active ? 'var(--paper-3)' : 'transparent',
 color: active ? 'var(--ink)' : 'var(--ink-2)',
 fontSize: 13
 }} className="gap-2 py-2 px-2 grid items-center w-full text-left" >
      <span className="kanji" style={{ fontSize: 13, width: 14,
                    color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
      <span className="inline-flex items-center" style={{ gap: 6 }}>{label}
        {alert && <span className="rounded-full bg-danger shrink-0" style={{ width: 6, height: 6 }} title="needs attention"/>}
      </span>
      {badge != null && (
        badgeTone
          ? <span className="mono font-semibold text-paper" style={{ fontSize: 10,
 background: badgeTone, borderRadius: 10, padding: '0 8px', lineHeight: '16px' }}>{badge}</span>
          : <span className="mono text-ink-3" style={{ fontSize: 11 }}>{badge}</span>
      )}
    </button>
    );
  };

  // Collapsible group: a header button that shows/hides its child NavItems.
  // Opens automatically when one of its children is the active section.
  const NavGroup = ({ kanji, label, count, items, alert }) => {
    const hasActive = items.some(it => it.id === section);
    const [open, setOpen] = oS(hasActive);
    oE(() => { if (hasActive) setOpen(true); }, [hasActive]);
    return (
      <div className="py-1 px-2" >
        <button onClick={() => setOpen(o => !o)}
 style={{ gridTemplateColumns: 'auto 1fr auto auto', fontSize: 13,
 borderRadius: 6
 }} className="gap-2 py-2 px-0 grid items-center w-full text-left bg-transparent text-ink-2" >
          <span className="kanji text-ink-3" style={{ fontSize: 13, width: 14 }}>{kanji}</span>
          <span className="inline-flex items-center" style={{ gap: 6 }}>{label}
            {alert && <span className="rounded-full bg-danger shrink-0" style={{ width: 6, height: 6 }} title="needs attention"/>}
          </span>
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>{count}</span>
          <span className="text-ink-3 inline-block text-center" style={{ fontSize: 9, width: 12,
 transform: open ? 'rotate(90deg)' : 'none',
 transition: 'transform .15s ease' }}>▶</span>
        </button>
        {open && (
          <div className="gap-1 pl-6 flex flex-col" >
            {items.map(it => <NavItem key={it.id} {...it}/>)}
          </div>
        )}
      </div>
    );
  };

  return (
    <aside className="py-6 px-3 gap-4 border-r bg-paper-2 flex flex-col overflow-auto" >
      <div className="px-1" >
        <Wordmark size={22}/>
      </div>

      <div>
        <div style={{ gap: 8 }} className="pt-0 pb-2 px-2 flex items-center" >
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>Observatory</span>
          <span className="flex-1" />
          {/* Focus — tame the surface on busy mornings to just what needs a decision */}
          <div className="flex bg-paper-3" style={{ borderRadius: 5, padding: 2 }}>
            {[['all', 'All'], ['focus', 'Focus']].map(([v, l]) => {
              const on = (v === 'focus') === focus;
              return (
                <button className="border-0 cursor-pointer" key={v} onClick={() => setFocus(v === 'focus')}
 style={{ fontSize: 10, padding: '4px 8px', borderRadius: 3,
 background: on ? 'var(--paper)' : 'transparent',
 color: on ? 'var(--ink)' : 'var(--ink-3)',
 fontFamily: 'inherit' }}>{l}</button>
              );
            })}
          </div>
        </div>
        <div className="gap-1 flex flex-col" >
          {/* Anchors — where every day starts */}
          <NavItem id="intake"    kanji="門" label="Intake"/>
          <NavItem id="home"      kanji="家" label="Today"/>
          <NavItem id="projects"  kanji="場" label="Projects"   badge={D.projects.active.length + D.projects.recent.length}/>
          {/* Needs you — the daily payoff: everything with a pending decision */}
          <Cluster label="Needs you"/>
          <NavItem id="insights"  kanji="今" label="Insights"   badge="6"/>
          <NavItem id="memories"  kanji="覚" label="Memories"   badge="7"
                   match={["memories","share-review","consolidation"]}/>
          <NavItem id="impact"    kanji="果" label="Impact"     badge="3" alert={true}
                   match={["impact","impact-alert"]}/>
          <NavItem id="traceability" kanji="巻" label="Traceability" badge="4"/>
          <NavItem id="upgrades"     kanji="贈" label="Upgrades"   badge="5"/>
          {/* Review & diagnostics — reached periodically, hidden in Focus */}
          {!focus &&
            <>
              <Cluster label="Review"/>
              <NavItem id="sessions"     kanji="録" label="Sessions"     badge="41"/>
              <NavItem id="libraries"    kanji="庫" label="Libraries"    badge="14"/>
              <NavItem id="instruments-playground" kanji="具" label="Instruments"
                       match={["instruments-playground","instruments-replay","instruments-health"]}/>
              <NavItem id="logs"      kanji="診" label="Logs"/>
            </>
          }
          {/* Settings — visited when something needs changing, hidden in Focus */}
          {!focus &&
            <div className="mt-2 pt-2 border-t" >
              <div className="gap-1 flex flex-col" >
                <NavItem id="dojo-connections" kanji="結" label="Dōjō"
                         match={["dojo-connections","dojo-sharing"]}/>
                <NavItem id="configure" kanji="調" label="Preferences"/>
              </div>
            </div>
          }
        </div>
      </div>

      <div className="flex-1" />

      <div style={{
 fontSize: 11, lineHeight: 1.6
 }} className="pt-2 pb-0 px-2 border-t text-ink-3" >
        <span className="mono">daemon · running</span><br/>
        <span className="text-ink-4" >last heartbeat 2s ago</span>
      </div>
    </aside>
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
    <div style={{ maxWidth: 520 }} className="p-16 mx-auto text-center" >
      <div className="kanji mb-3 text-accent" style={{
 fontSize: 56, opacity: 0.5
 }}>{L.k}</div>
      <h1 className="display mt-0 mb-2 font-light" style={{ fontSize: 28 }}>
        {L.t}
      </h1>
      <p style={{ fontSize: 13 }} className="mt-0 mb-6 text-ink-3" >{L.s}</p>
      <button onClick={onBack}
 style={{
 fontSize: 13, border: 'var(--ink-line)',
 borderRadius: 5 }} className="py-2 px-4 text-ink-2" >
        ← Today
      </button>
    </div>
  );
}

function FirstEntryToast({ onDismiss, mode }) {
  // Read current settings to seed the "don't show again" checkbox.
  const init = (() => {
    try {
      const raw = localStorage.getItem("sensei-settings");
      if (!raw) return false;
      return JSON.parse(raw).showWelcome === false;
    } catch { return false; }
  })();
  const [hide, setHide] = oS(init);

  const toggleHide = () => {
    const next = !hide;
    setHide(next);
    try {
      const raw = localStorage.getItem("sensei-settings");
      const cur = raw ? JSON.parse(raw) : {};
      const updated = { ...cur, showWelcome: !next };
      localStorage.setItem("sensei-settings", JSON.stringify(updated));
      window.dispatchEvent(new CustomEvent("sensei-settings-changed"));
    } catch {}
  };

  return (
    <div style={{ top: 24, left: '50%', transform: 'translateX(-50%)', borderRadius: 10, zIndex: 20,
 animation: 'toast-in .45s ease-out'
 }} className="gap-4 py-3 pl-4 pr-6 absolute bg-ink text-paper flex items-center shadow-lg" >
      <span className="kanji text-accent" style={{ fontSize: 22 }}>礼</span>
      <div>
        <div className="display mb-1 font-normal" style={{ fontSize: 13 }}>
          The observatory is open.
        </div>
        <div style={{ fontSize: 11, color: 'var(--edge)', opacity: 0.75 }}>
          {mode === "early"
            ? "Come back after a few sessions. Sensei is still listening."
            : "Three projects · 81 sessions watched · 3 teachings adopted so far."}
        </div>
      </div>
      <label style={{
 fontSize: 11, color: 'var(--edge)',
 opacity: 0.85, borderLeft: '1px solid var(--on-primary-faint)'
 }} className="gap-1 ml-2 pl-3 flex items-center cursor-pointer" >
        <input className="cursor-pointer" type="checkbox" checked={hide} onChange={toggleHide}
 style={{ accentColor: 'var(--accent)' }}/>
        Don't show again
      </label>
      <button onClick={onDismiss} style={{
 fontSize: 11 }} className="py-1 px-2 ml-1 text-ink-4" >dismiss</button>
      <style>{`@keyframes toast-in {
        from { opacity: 0; transform: translate(-50%, -12px) }
        to { opacity: 1; transform: translateX(-50%) }
      }`}</style>
    </div>
  );
}

// Guided first session — replaces the slim notice on the early Projects
// landing. The "quiet first days" gap: after the scan there's nothing to DO
// until sessions accrue. This makes the next action concrete — go run one
// real session so sensei has something to watch — and shows how close the
// first lesson is (≈ 3 sessions). Once sessions start landing it gives way
// to the normal "still listening" early signals.
function FirstSessionGuide({ onOpenToday, onOpenProject }) {
  const fs = window.OBS_DATA.firstSession;
  const [copied, setCopied] = oS(false);
  const copyCmd = () => {
    try { navigator.clipboard && navigator.clipboard.writeText(fs.command); } catch {}
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const steps = [
    { n: "場", t: "Pick a project",
      body: (
        <>Any of the {fs.projectsFound} sensei just found.{" "}
          <button className="text-accent p-0" onClick={() => onOpenProject && onOpenProject(fs.suggested.id)}
 style={{ fontSize: 12,
 borderBottom: '1px solid color-mix(in srgb, var(--accent) 40%, transparent)' }}>
            {fs.suggested.name}
          </button>{" "}is the busiest.</>
      ) },
    { n: "連", t: `Open ${fs.assistant}`,
      body: (
        <div className="gap-2 mt-1 flex items-center" >
          <code className="mono text-ink-2 bg-paper border border-paper-edge whitespace-nowrap overflow-hidden text-ellipsis" style={{
 fontSize: 11, borderRadius: 4, padding: '4px 8px', maxWidth: 230
 }}>{fs.command}</code>
          <button onClick={copyCmd}
 style={{ fontSize: 11, color: copied ? 'var(--success)' : 'var(--ink-3)', borderRadius: 4 }}
 className="py-1 px-2 border border-paper-edge whitespace-nowrap" >
            {copied ? "copied ✓" : "copy"}
          </button>
        </div>
      ) },
    { n: "観", t: "Just work",
      body: "Code as you normally would. Sensei watches in the background — nothing to launch." }
  ];

  const ticks = Array.from({ length: fs.target });

  return (
    <div 
 className="py-6 px-12 bg-accent-soft border-b" >
      <div style={{ gridTemplateColumns: '300px 1fr 196px', maxWidth: 1120 }} className="gap-12 mx-auto grid items-start" >

        {/* ── Framing ── */}
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-accent uppercase" >
            Your first session
          </div>
          <div className="gap-3 flex items-start" >
            <span className="kanji text-accent" style={{ fontSize: 34, lineHeight: 1 }}>稽</span>
            <div>
              <h2 className="display m-0 font-normal" style={{ fontSize: 20,
 letterSpacing: '-0.01em', lineHeight: 1.2 }}>
                Give sensei something<br/>to watch.
              </h2>
            </div>
          </div>
          <p style={{ fontSize: 13, lineHeight: 1.6 }} className="mt-3 mb-0 text-ink-2" >
            It learns from how you and your assistant actually work — there's nothing
            to teach yet. Run a few real sessions and the first lesson forms.
          </p>
        </div>

        {/* ── The three steps ── */}
        <div className="gap-3 flex flex-col" >
          {steps.map((s, i) => (
            <div key={i} style={{ gridTemplateColumns: 'auto 1fr' }} className="gap-3 grid items-start" >
              <span className="rounded-full bg-paper border border-paper-edge flex items-center justify-center shrink-0" style={{ width: 26, height: 26 }}>
                <span className="kanji text-accent" style={{ fontSize: 14, lineHeight: 1 }}>{s.n}</span>
              </span>
              <div className="min-w-0" >
                <div className="text-ink font-medium" style={{ fontSize: 13 }}>{s.t}</div>
                <div style={{ fontSize: 12, lineHeight: 1.5 }} className="mt-1 text-ink-2" >
                  {s.body}
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* ── Progress toward the first lesson ── */}
        <div className="pl-8 border-l" >
          <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
            Sessions watched
          </div>
          <div className="gap-1 mt-1 flex items-baseline" >
            <span className="display font-normal text-ink" style={{ fontSize: 34, lineHeight: 1 }}>{fs.watched}</span>
            <span className="text-ink-3" style={{ fontSize: 15 }}>/ {fs.target}</span>
          </div>
          <div className="gap-1 mt-2 flex" >
            {ticks.map((_, i) => (
              <span className="flex-1" key={i} style={{ height: 4, borderRadius: 2,
 background: i < fs.watched ? 'var(--accent)' : 'var(--edge)' }}/>
            ))}
          </div>
          <div style={{ fontSize: 11, lineHeight: 1.5 }} className="mt-2 text-ink-3" >
            The first lesson tends to form around the third session.
          </div>
          <div className="gap-2 mt-3 flex items-center" >
            <span className="rounded-full bg-success" style={{ width: 6, height: 6,
 animation: 'fsPulse 1.6s ease-in-out infinite' }}/>
            <span className="mono text-ink-2" style={{ fontSize: 11 }}>daemon listening</span>
          </div>
          <button onClick={onOpenToday}
 style={{ fontSize: 11 }} className="mt-3 p-0 text-ink-3" >
            Peek at Today →
          </button>
          <style>{`@keyframes fsPulse { 0%,100% { opacity: 1 } 50% { opacity: 0.3 } }`}</style>
        </div>
      </div>
    </div>
  );
}

// Slim banner shown above Projects while sensei is still early (no insights
// yet). Explains why Today is quiet and that it will open on its own —
// addresses the "where did my dashboard go?" risk of landing on Projects.
function EarlyProjectsNotice({ onOpenToday }) {
  return (
    <div style={{ gap: 12 }} className="py-3 px-12 flex items-center bg-accent-soft border-b" >
      <span className="kanji text-accent" style={{ fontSize: 17, lineHeight: 1 }}>観</span>
      <div className="flex-1 min-w-0" >
        <span className="text-ink" style={{ fontSize: 13 }}>
          Sensei is still listening.
        </span>
        <span style={{ fontSize: 13 }} className="ml-2 text-ink-2" >
          Here are the projects it found. Today’s brief opens on its own once the first insights form — a few sessions away.
        </span>
      </div>
      <button onClick={onOpenToday}
 style={{ fontSize: 11, border: 'var(--ink-line)',
 borderRadius: 5 }} className="py-1 px-3 text-ink-2 whitespace-nowrap" >
        Peek at Today →
      </button>
    </div>
  );
}

// ─── Harness for design canvas ───────────────────────────────
function ObservatoryDailyApp()       { return <ObservatoryDaily stateMode="mature"/>; }
function ObservatoryEarlyApp()       { return <ObservatoryDaily stateMode="early"/>;  }
// First entry from setup lands you in the "early · still listening" state —
// the system has just been configured, no sessions watched yet, sensei is
// quiet on purpose. This matches what the user sees if they finish the
// wizard inside the actual observatory shell (Configure → Done) and is
// the dedicated artboard for that moment.
function ObservatoryEntryApp()       { return <ObservatoryDaily stateMode="early"  firstEntry={true}/>; }
function ObservatoryIntakeApp()      { return <ObservatoryDaily stateMode="mature" initialSection="intake"/>; }

Object.assign(window, {
  ObservatoryDaily, ObservatoryDailyApp, ObservatoryEarlyApp, ObservatoryEntryApp, ObservatoryIntakeApp
});
