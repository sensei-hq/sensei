// Three variations of the Project page — Top-tabs / Left-rail / Long-scroll.
// All three share ProjHeader + ProjOverview + ProjGraphLens + ProjPatterns + ProjSessions + ProjSettings.

const { useState: ppS, useEffect: ppE, useRef: ppR } = React;

const PROJ_SECTIONS = [
  { id: "overview",  kanji: "全", label: "Overview"  },
  { id: "graph",     kanji: "構", label: "Graph"     },
  { id: "patterns",  kanji: "紋", label: "Patterns"  },
  { id: "sessions",  kanji: "録", label: "Sessions"  },
  { id: "settings",  kanji: "識", label: "About"     }
];

function renderSection(id, project, openAction) {
  switch (id) {
    case "overview":     return <ProjOverview    project={project} openAction={openAction}/>;
    case "graph":        return <ProjGraphLens   project={project}/>;
    case "patterns":     return <ProjPatterns    openAction={openAction}/>;
    case "sessions":     return <ProjSessions/>;
    case "settings":     return <ProjSettingsV2  project={project}/>;
    default:             return null;
  }
}

function useActionDrawer() {
  const [drawer, setDrawer] = ppS(null);
  const openAction = (rec, mode) => setDrawer({ rec, mode });
  const close = () => setDrawer(null);
  return { drawer, openAction, close };
}

// ═══════════════════════════════════════════════════════════
// Variation A — Top tabs (classic, scannable)
// ═══════════════════════════════════════════════════════════
function ProjectPageTopTabs({ embedded = false, onBack, projectId } = {}) {
  const projects = window.PROJECT_DATA.projects;
  const project = projects[projectId || window.PROJECT_DATA.active] || projects[window.PROJECT_DATA.active];
  const [sec, setSec] = ppS("overview");
  const { drawer, openAction, close } = useActionDrawer();

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Project · Top tabs"
 >
      {!embedded && <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>}
      <ProjHeader project={project} onBack={onBack || (() => {})} showBack={!!embedded && !!onBack}/>

      {/* Tab bar */}
      <div className="gap-1 px-12 border-b flex bg-paper" >
        {PROJ_SECTIONS.map(s => {
          const on = sec === s.id;
          return (
            <button key={s.id} onClick={() => setSec(s.id)}
 style={{
 fontSize: 13,
 borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
 color: on ? 'var(--ink)' : 'var(--ink-3)',
 marginBottom: -1
 }} className="gap-2 py-3 px-4 inline-flex items-center" >
              <span className="kanji" style={{ fontSize: 13,
                            color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
              {s.label}
            </button>
          );
        })}
      </div>

      <main className="flex-1 overflow-auto" >
        {renderSection(sec, project, openAction)}
      </main>

      {drawer && <ProjActionDrawer rec={drawer.rec} mode={drawer.mode} onClose={close}/>}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════
// Variation B — Left inner rail (more content real estate)
// ═══════════════════════════════════════════════════════════
function ProjectPageLeftRail() {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  const [sec, setSec] = ppS("overview");
  const { drawer, openAction, close } = useActionDrawer();

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Project · Left rail"
 >
      <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>
      <ProjHeader project={project} onBack={() => {}}/>

      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '180px 1fr' }}>
        <aside className="py-6 px-3 gap-1 border-r bg-paper-2 flex flex-col" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-0 pb-2 px-2 text-ink-3 uppercase" >
            This project
          </div>
          {PROJ_SECTIONS.map(s => {
            const on = sec === s.id;
            return (
              <button key={s.id} onClick={() => setSec(s.id)}
 style={{ gridTemplateColumns: 'auto 1fr', borderRadius: 5,
 background: on ? 'var(--paper)' : 'transparent',
 color: on ? 'var(--ink)' : 'var(--ink-2)', fontSize: 13
 }} className="gap-2 py-2 px-2 grid items-center text-left" >
                <span className="kanji" style={{ fontSize: 13, width: 14,
                              color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
                <span>{s.label}</span>
              </button>
            );
          })}
          <div style={{ height: 12 }}/>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-0 pb-2 px-2 text-ink-3 uppercase" >Quick</div>
          <button style={{
 fontSize: 13 }} className="py-2 px-2 text-left text-ink-2" >◌ open in terminal</button>
          <button style={{
 fontSize: 13 }} className="py-2 px-2 text-left text-ink-2" >◌ start session</button>
          <button style={{
 fontSize: 13 }} className="py-2 px-2 text-left text-ink-2" >◌ scan now</button>
        </aside>

        <main className="overflow-auto" >
          {renderSection(sec, project, openAction)}
        </main>
      </div>

      {drawer && <ProjActionDrawer rec={drawer.rec} mode={drawer.mode} onClose={close}/>}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════
// Variation C — Long scroll, anchored section links (most zen)
// ═══════════════════════════════════════════════════════════
function ProjectPageLongScroll() {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  const [active, setActive] = ppS("overview");
  const { drawer, openAction, close } = useActionDrawer();
  const refs = { overview: ppR(), graph: ppR(), patterns: ppR(), sessions: ppR(), settings: ppR() };
  const scrollRef = ppR();

  ppE(() => {
    const root = scrollRef.current; if (!root) return;
    const onScroll = () => {
      const sTop = root.scrollTop + 60;
      let hit = "overview";
      for (const s of PROJ_SECTIONS) {
        const el = refs[s.id].current;
        if (el && el.offsetTop <= sTop) hit = s.id;
      }
      setActive(hit);
    };
    root.addEventListener('scroll', onScroll);
    return () => root.removeEventListener('scroll', onScroll);
  }, []);

  const goto = (id) => {
    const el = refs[id].current; if (!el || !scrollRef.current) return;
    scrollRef.current.scrollTo({ top: el.offsetTop - 20, behavior: 'smooth' });
  };

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Project · Long scroll"
 >
      <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>
      <ProjHeader project={project} onBack={() => {}}/>

      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '1fr 180px' }}>
        <main className="overflow-auto" ref={scrollRef} style={{ scrollBehavior: 'smooth' }}>
          {PROJ_SECTIONS.map(s => (
            <section key={s.id} ref={refs[s.id]}>
              <div style={{
 borderTop: s.id !== "overview" ? 'var(--hairline)' : 'none',
 marginTop: s.id !== "overview" ? 6 : 0
 }} className="gap-3 pt-8 pb-0 px-12 flex items-baseline" >
                <span className="kanji text-accent" style={{ fontSize: 22 }}>{s.kanji}</span>
                <h2 className="display m-0 font-normal" style={{
 fontSize: 22, letterSpacing: '-0.01em'
 }}>{s.label}</h2>
              </div>
              {renderSection(s.id, project, openAction)}
            </section>
          ))}
          <div style={{ height: 60 }}/>
        </main>

        <aside style={{ top: 0, alignSelf: 'start'
 }} className="py-6 px-4 gap-1 border-l bg-paper-2 flex flex-col sticky" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-0 pb-3 px-2 text-ink-3 uppercase" >On this page</div>
          {PROJ_SECTIONS.map(s => {
            const on = active === s.id;
            return (
              <button key={s.id} onClick={() => goto(s.id)}
 style={{
 fontSize: 13, gridTemplateColumns: 'auto 1fr',
 color: on ? 'var(--ink)' : 'var(--ink-3)',
 borderLeft: on ? '2px solid var(--accent)' : '2px solid transparent'
 }} className="py-2 px-2 gap-2 pl-3 text-left grid items-center" >
                <span className="kanji" style={{ fontSize: 13, width: 12,
                              color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
                <span>{s.label}</span>
              </button>
            );
          })}
        </aside>
      </div>

      {drawer && <ProjActionDrawer rec={drawer.rec} mode={drawer.mode} onClose={close}/>}
    </div>
  );
}

Object.assign(window, {
  ProjectPageTopTabs, ProjectPageLeftRail, ProjectPageLongScroll,
  ProjectPageSidebar,
  ProjectSettingsV1Page, ProjectSettingsV2Page
});

// ═══════════════════════════════════════════════════════════
// Variation D — Project window with its own LEFT SIDEBAR
// (replaces the top-tabs pattern; matches the perspective-split A
//  layout where the project window has its own complete sidebar with
//  Overview · Sessions · Memories · Traceability · Libraries ·
//  Instruments · Patterns · Impact · Logs · Settings)
// ═══════════════════════════════════════════════════════════
function ProjectPageSidebar({ initialSection = "overview", embedded = false, onBack, onSwitchProject, projectId, state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="雲"
    emptyTitle="This project has no data yet"
    emptyHint="Point sensei at this project and run a session — its overview, sessions, memories and impact fill in here."
    errorHint="Couldn't open this project window. Try again." onRetry={() => {}} />;
  const project = window.PROJECT_DATA.projects[projectId || window.PROJECT_DATA.active]
                  || window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  const [sec, setSec] = ppS(initialSection);
  const { drawer, openAction, close } = useActionDrawer();

  // Every section now renders a simplified, in-context preview.
  const renderProjectSection = (id) => {
    switch (id) {
      case "overview":     return <ProjOverviewLite   project={project} openAction={openAction}/>;
      case "sessions":     return <SessionsDigestZen
                                      projectFilter={project.id}
                                      projectLabel={project.name}/>;
      case "memories":     return <ProjMemoriesLite   project={project}/>;
      case "traceability": return <ProjTraceabilityLite project={project}/>;
      case "libraries":    return <ProjLibrariesLite  project={project}/>;
      case "instruments":  return <ProjInstrumentsLite project={project}/>;
      case "patterns":     return <ProjPatterns       openAction={openAction}/>;
      case "impact":       return <ProjImpactLite     project={project}/>;
      case "about":        return <ProjAboutPane     project={project}/>;
      default:             return <ProjOverviewLite   project={project} openAction={openAction}/>;
    }
  };

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Project · Sidebar"
 >
      {!embedded && (
        <PerspectiveChrome
          title={`先生  ·  ${project.name}`}
          subtitle="project window"
          accent="var(--accent)"/>
      )}

      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '220px 1fr' }}>
        {/* The same sidebar used in the perspective-split — drives section selection */}
        <ProjectSidebarRouted project={project} active={sec} onChange={setSec}
                              onSwitchProject={onSwitchProject}/>

        <main className="overflow-auto relative" >
          {renderProjectSection(sec)}
        </main>
      </div>

      {drawer && <ProjActionDrawer rec={drawer.rec} mode={drawer.mode} onClose={close}/>}
    </div>
  );
}

// Routed wrapper around the existing ProjectSidebar (so clicking a section
// changes the right pane, instead of just rendering a static "active" mark).
function ProjectSidebarRouted({ project, active, onChange, onSwitchProject }) {
  // The sidebar sections list is defined in perspective-split.jsx as
  // PROJ_SIDEBAR_SECTIONS; we re-render it here with click-handlers wired.
  return (
    <aside style={{ boxSizing: 'border-box'
 }} className="py-6 px-3 gap-4 border-r bg-paper-2 flex flex-col overflow-auto h-full" >
      <div className="px-1" >
        <KanjiHeader variant="h2" kanji={project.kanji} eyebrow="Project" title={project.name}/>
        <div className="mono mt-2 text-ink-3" style={{ fontSize: 11 }}>
          {project.client || "lumen-systems"}
        </div>
        <button onClick={onSwitchProject}
 style={{
 fontSize: 11, borderRadius: 4 }} className="mt-2 py-1 px-2 text-ink-3 border border-paper-edge bg-transparent cursor-pointer" >
          ⇆ switch project
        </button>
      </div>

      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-0 pb-2 px-2 text-ink-3 uppercase" >This project</div>
        <div className="gap-1 flex flex-col" >
          {[
            { id: "overview",     kanji: "全", label: "Overview"    },
            { id: "sessions",     kanji: "録", label: "Sessions",     badge: "28" },
            { id: "memories",     kanji: "覚", label: "Memories",     badge: "11" },
            { id: "traceability", kanji: "巻", label: "Traceability", badge: "4"  },
            { id: "libraries",    kanji: "庫", label: "Libraries",    badge: "5"  },
            { id: "instruments",  kanji: "具", label: "Instruments",  badge: "7"  },
            { id: "patterns",     kanji: "紋", label: "Patterns",     badge: "3"  },
            { id: "impact",       kanji: "果", label: "Impact",       badge: "2"  },
            { id: "about",        kanji: "識", label: "About"      },
          ].map(s => (
            <button key={s.id} onClick={() => onChange(s.id)}
 style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 6,
 background: s.id === active ? 'var(--paper-3)' : 'transparent',
 color: s.id === active ? 'var(--ink)' : 'var(--ink-2)',
 fontSize: 13 }} className="gap-2 py-2 px-2 grid items-center w-full text-left cursor-pointer border-0" >
              <span className="kanji" style={{ fontSize: 13, width: 14,
                            color: s.id === active ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
              <span>{s.label}</span>
              {s.badge != null && (
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>{s.badge}</span>
              )}
            </button>
          ))}
        </div>
      </div>

      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-0 pb-2 px-2 text-ink-3 uppercase" >Health</div>
        <div style={{
 fontSize: 11 }} className="gap-1 px-2 text-ink-3 flex flex-col" >
          <div className="flex justify-between" >
            <span>FTR · 14d</span>
            <span className="mono" style={{ color: project.warn ? 'var(--warning)' : 'var(--ink)' }}>
              {Math.round((project.ftr || 0.78) * 100)}%
            </span>
          </div>
          <div className="flex justify-between" >
            <span>Sessions · 7d</span>
            <span className="mono text-ink-2" >{project.sessions7d || 28}</span>
          </div>
          <div className="flex justify-between" >
            <span>Drift watch</span>
            <span className="mono text-warning" >3 docs</span>
          </div>
        </div>
      </div>

      <div className="flex-1" />

      <div style={{
 fontSize: 11, lineHeight: 1.6
 }} className="pt-2 pb-0 px-2 border-t text-ink-3" >
        <span className="mono">scoped to this project</span>
      </div>
    </aside>
  );
}

// ═══════════════════════════════════════════════════════════
// Settings-focused artboards — show the settings tab alone
// so variant A / B can be compared side by side.
// ═══════════════════════════════════════════════════════════
function ProjectSettingsV1Page() {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Project · Settings A"
 >
      <TauriChrome title={`Sensei  先生  ·  ${project.name} · settings`}/>
      <ProjHeader project={project} onBack={() => {}}/>
      <div className="gap-1 px-12 border-b flex bg-paper" >
        {PROJ_SECTIONS.map(s => {
          const on = s.id === "settings";
          return (
            <div key={s.id}
 style={{
 fontSize: 13,
 borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
 color: on ? 'var(--ink)' : 'var(--ink-3)',
 marginBottom: -1
 }} className="gap-2 py-3 px-4 inline-flex items-center" >
              <span className="kanji" style={{ fontSize: 13,
                            color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
              {s.label}
            </div>
          );
        })}
      </div>
      <main className="flex-1 overflow-auto" >
        <ProjSettings project={project}/>
      </main>
    </div>
  );
}

function ProjectSettingsV2Page() {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Project · Settings B"
 >
      <TauriChrome title={`Sensei  先生  ·  ${project.name} · settings`}/>
      <ProjHeader project={project} onBack={() => {}}/>
      <div className="gap-1 px-12 border-b flex bg-paper" >
        {PROJ_SECTIONS.map(s => {
          const on = s.id === "settings";
          return (
            <div key={s.id}
 style={{
 fontSize: 13,
 borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
 color: on ? 'var(--ink)' : 'var(--ink-3)',
 marginBottom: -1
 }} className="gap-2 py-3 px-4 inline-flex items-center" >
              <span className="kanji" style={{ fontSize: 13,
                            color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
              {s.label}
            </div>
          );
        })}
      </div>
      <main className="flex-1 overflow-hidden" >
        <ProjSettingsV2 project={project}/>
      </main>
    </div>
  );
}
