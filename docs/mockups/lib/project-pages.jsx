// Three variations of the Project page — Top-tabs / Left-rail / Long-scroll.
// All three share ProjHeader + ProjOverview + ProjGraphLens + ProjPatterns + ProjSessions + ProjSettings.

const { useState: ppS, useEffect: ppE, useRef: ppR } = React;

const PROJ_SECTIONS = [
  { id: "overview",  kanji: "全", label: "Overview"  },
  { id: "graph",     kanji: "構", label: "Graph"     },
  { id: "patterns",  kanji: "紋", label: "Patterns"  },
  { id: "sessions",  kanji: "録", label: "Sessions"  },
  { id: "settings",  kanji: "設", label: "Settings"  }
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
    <div className="sensei" data-screen-label="Project · Top tabs"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      {!embedded && <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>}
      <ProjHeader project={project} onBack={onBack || (() => {})} showBack={!!embedded && !!onBack}/>

      {/* Tab bar */}
      <div style={{ padding: '0 48px', borderBottom: 'var(--hairline)',
                    display: 'flex', gap: 4, background: 'var(--paper)' }}>
        {PROJ_SECTIONS.map(s => {
          const on = sec === s.id;
          return (
            <button key={s.id} onClick={() => setSec(s.id)}
                    style={{
                      padding: '12px 16px 12px', fontSize: 13,
                      display: 'inline-flex', alignItems: 'center', gap: 8,
                      borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
                      color: on ? 'var(--ink)' : 'var(--ink-3)',
                      marginBottom: -1
                    }}>
              <span className="kanji" style={{ fontSize: 13,
                            color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
              {s.label}
            </button>
          );
        })}
      </div>

      <main style={{ flex: 1, overflow: 'auto' }}>
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
    <div className="sensei" data-screen-label="Project · Left rail"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>
      <ProjHeader project={project} onBack={() => {}}/>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '180px 1fr', minHeight: 0 }}>
        <aside style={{ borderRight: 'var(--hairline)', padding: '24px 12px',
                         background: 'var(--paper-2)', display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', padding: '0 8px 8px' }}>
            This project
          </div>
          {PROJ_SECTIONS.map(s => {
            const on = sec === s.id;
            return (
              <button key={s.id} onClick={() => setSec(s.id)}
                      style={{
                        display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 8,
                        alignItems: 'center', padding: '8px 8px', borderRadius: 5,
                        textAlign: 'left',
                        background: on ? 'var(--paper)' : 'transparent',
                        color: on ? 'var(--ink)' : 'var(--ink-2)', fontSize: 13
                      }}>
                <span className="kanji" style={{ fontSize: 13, width: 14,
                              color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
                <span>{s.label}</span>
              </button>
            );
          })}
          <div style={{ height: 12 }}/>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', padding: '0 8px 8px' }}>Quick</div>
          <button style={{ padding: '8px 8px', fontSize: 13, textAlign: 'left',
                            color: 'var(--ink-2)' }}>◌ open in terminal</button>
          <button style={{ padding: '8px 8px', fontSize: 13, textAlign: 'left',
                            color: 'var(--ink-2)' }}>◌ start session</button>
          <button style={{ padding: '8px 8px', fontSize: 13, textAlign: 'left',
                            color: 'var(--ink-2)' }}>◌ scan now</button>
        </aside>

        <main style={{ overflow: 'auto' }}>
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
    <div className="sensei" data-screen-label="Project · Long scroll"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>
      <ProjHeader project={project} onBack={() => {}}/>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 180px', minHeight: 0 }}>
        <main ref={scrollRef} style={{ overflow: 'auto', scrollBehavior: 'smooth' }}>
          {PROJ_SECTIONS.map(s => (
            <section key={s.id} ref={refs[s.id]}>
              <div style={{ padding: '32px 48px 0',
                             display: 'flex', alignItems: 'baseline', gap: 12,
                             borderTop: s.id !== "overview" ? 'var(--hairline)' : 'none',
                             marginTop: s.id !== "overview" ? 6 : 0 }}>
                <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>{s.kanji}</span>
                <h2 className="display" style={{ fontSize: 22, fontWeight: 400,
                              margin: 0, letterSpacing: '-0.01em' }}>{s.label}</h2>
              </div>
              {renderSection(s.id, project, openAction)}
            </section>
          ))}
          <div style={{ height: 60 }}/>
        </main>

        <aside style={{ borderLeft: 'var(--hairline)', padding: '24px 16px',
                         background: 'var(--paper-2)',
                         display: 'flex', flexDirection: 'column', gap: 4,
                         position: 'sticky', top: 0, alignSelf: 'start' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', padding: '0 8px 12px' }}>On this page</div>
          {PROJ_SECTIONS.map(s => {
            const on = active === s.id;
            return (
              <button key={s.id} onClick={() => goto(s.id)}
                      style={{
                        padding: '8px 8px', fontSize: 13, textAlign: 'left',
                        display: 'grid', gridTemplateColumns: 'auto 1fr',
                        gap: 8, alignItems: 'center',
                        color: on ? 'var(--ink)' : 'var(--ink-3)',
                        borderLeft: on ? '2px solid var(--accent)' : '2px solid transparent',
                        paddingLeft: 12
                      }}>
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
function ProjectPageSidebar({ initialSection = "overview", embedded = false, onBack, onSwitchProject } = {}) {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
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
    <div className="sensei" data-screen-label="Project · Sidebar"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      {!embedded && (
        <PerspectiveChrome
          title={`先生  ·  ${project.name}`}
          subtitle="project window"
          accent="var(--accent)"/>
      )}

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '220px 1fr', minHeight: 0 }}>
        {/* The same sidebar used in the perspective-split — drives section selection */}
        <ProjectSidebarRouted project={project} active={sec} onChange={setSec}
                              onSwitchProject={onSwitchProject}/>

        <main style={{ overflow: 'auto', position: 'relative' }}>
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
    <aside style={{ borderRight: 'var(--hairline)', padding: '24px 12px',
                     background: 'var(--paper-2)',
                     display: 'flex', flexDirection: 'column', gap: 16,
                     overflow: 'auto', height: '100%', boxSizing: 'border-box' }}>
      <div style={{ padding: '0 4px' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>Project</div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span className="kanji" style={{ fontSize: 28, color: 'var(--accent)', lineHeight: 1 }}>
            {project.kanji}
          </span>
          <div style={{ minWidth: 0 }}>
            <div className="display" style={{ fontSize: 15, color: 'var(--ink)',
                          letterSpacing: '-0.01em', lineHeight: 1.1 }}>{project.name}</div>
            <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
              {project.client || "lumen-systems"}
            </div>
          </div>
        </div>
        <button onClick={onSwitchProject}
                style={{ marginTop: 8, fontSize: 11, color: 'var(--ink-3)',
                          padding: '4px 8px', border: 'var(--hairline)', borderRadius: 4,
                          background: 'transparent', cursor: 'pointer' }}>
          ⇆ switch project
        </button>
      </div>

      <div>
        <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', padding: '0 8px 8px' }}>This project</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {[
            { id: "overview",     kanji: "全", label: "Overview"    },
            { id: "sessions",     kanji: "刻", label: "Sessions",     badge: "28" },
            { id: "memories",     kanji: "覚", label: "Memories",     badge: "11" },
            { id: "traceability", kanji: "巻", label: "Traceability", badge: "4"  },
            { id: "libraries",    kanji: "庫", label: "Libraries",    badge: "5"  },
            { id: "instruments",  kanji: "具", label: "Instruments",  badge: "7"  },
            { id: "patterns",     kanji: "紋", label: "Patterns",     badge: "3"  },
            { id: "impact",       kanji: "果", label: "Impact",       badge: "2"  },
            { id: "about",        kanji: "識", label: "About"      },
          ].map(s => (
            <button key={s.id} onClick={() => onChange(s.id)}
                    style={{
                      display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 8,
                      alignItems: 'center', width: '100%',
                      padding: '8px 8px', borderRadius: 6, textAlign: 'left',
                      background: s.id === active ? 'var(--paper-3)' : 'transparent',
                      color: s.id === active ? 'var(--ink)' : 'var(--ink-2)',
                      fontSize: 13, cursor: 'pointer', border: 'none'
                    }}>
              <span className="kanji" style={{ fontSize: 13, width: 14,
                            color: s.id === active ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
              <span>{s.label}</span>
              {s.badge != null && (
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.badge}</span>
              )}
            </button>
          ))}
        </div>
      </div>

      <div>
        <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', padding: '0 8px 8px' }}>Health</div>
        <div style={{ padding: '0 8px', fontSize: 11, color: 'var(--ink-3)',
                       display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span>FTR · 14d</span>
            <span className="mono" style={{ color: project.warn ? 'var(--warning)' : 'var(--ink)' }}>
              {Math.round((project.ftr || 0.78) * 100)}%
            </span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span>Sessions · 7d</span>
            <span className="mono" style={{ color: 'var(--ink-2)' }}>{project.sessions7d || 28}</span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span>Drift watch</span>
            <span className="mono" style={{ color: 'var(--warning)' }}>3 docs</span>
          </div>
        </div>
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ padding: '8px 8px 0', borderTop: 'var(--hairline)',
                     fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.6 }}>
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
    <div className="sensei" data-screen-label="Project · Settings A"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <TauriChrome title={`Sensei  先生  ·  ${project.name} · settings`}/>
      <ProjHeader project={project} onBack={() => {}}/>
      <div style={{ padding: '0 48px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 4, background: 'var(--paper)' }}>
        {PROJ_SECTIONS.map(s => {
          const on = s.id === "settings";
          return (
            <div key={s.id}
                 style={{ padding: '12px 16px 12px', fontSize: 13,
                           display: 'inline-flex', alignItems: 'center', gap: 8,
                           borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
                           color: on ? 'var(--ink)' : 'var(--ink-3)',
                           marginBottom: -1 }}>
              <span className="kanji" style={{ fontSize: 13,
                            color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
              {s.label}
            </div>
          );
        })}
      </div>
      <main style={{ flex: 1, overflow: 'auto' }}>
        <ProjSettings project={project}/>
      </main>
    </div>
  );
}

function ProjectSettingsV2Page() {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  return (
    <div className="sensei" data-screen-label="Project · Settings B"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <TauriChrome title={`Sensei  先生  ·  ${project.name} · settings`}/>
      <ProjHeader project={project} onBack={() => {}}/>
      <div style={{ padding: '0 48px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 4, background: 'var(--paper)' }}>
        {PROJ_SECTIONS.map(s => {
          const on = s.id === "settings";
          return (
            <div key={s.id}
                 style={{ padding: '12px 16px 12px', fontSize: 13,
                           display: 'inline-flex', alignItems: 'center', gap: 8,
                           borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
                           color: on ? 'var(--ink)' : 'var(--ink-3)',
                           marginBottom: -1 }}>
              <span className="kanji" style={{ fontSize: 13,
                            color: on ? 'var(--accent)' : 'var(--ink-4)' }}>{s.kanji}</span>
              {s.label}
            </div>
          );
        })}
      </div>
      <main style={{ flex: 1, overflow: 'hidden' }}>
        <ProjSettingsV2 project={project}/>
      </main>
    </div>
  );
}
