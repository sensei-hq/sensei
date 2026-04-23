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
    case "overview": return <ProjOverview    project={project} openAction={openAction}/>;
    case "graph":    return <ProjGraphLens   project={project}/>;
    case "patterns": return <ProjPatterns    openAction={openAction}/>;
    case "sessions": return <ProjSessions/>;
    case "settings": return <ProjSettings    project={project}/>;
    default:         return null;
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
function ProjectPageTopTabs() {
  const project = window.PROJECT_DATA.projects[window.PROJECT_DATA.active];
  const [sec, setSec] = ppS("overview");
  const { drawer, openAction, close } = useActionDrawer();

  return (
    <div className="sensei" data-screen-label="Project · Top tabs"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <TauriChrome title={`Sensei  先生  ·  ${project.name}`}/>
      <ProjHeader project={project} onBack={() => {}}/>

      {/* Tab bar */}
      <div style={{ padding: '0 44px', borderBottom: 'var(--hairline)',
                    display: 'flex', gap: 2, background: 'var(--paper)' }}>
        {PROJ_SECTIONS.map(s => {
          const on = sec === s.id;
          return (
            <button key={s.id} onClick={() => setSec(s.id)}
                    style={{
                      padding: '14px 18px 12px', fontSize: 12.5,
                      display: 'inline-flex', alignItems: 'center', gap: 8,
                      borderBottom: on ? '2px solid var(--shu)' : '2px solid transparent',
                      color: on ? 'var(--sumi)' : 'var(--sumi-3)',
                      marginBottom: -1
                    }}>
              <span className="kanji" style={{ fontSize: 12,
                            color: on ? 'var(--shu)' : 'var(--sumi-4)' }}>{s.kanji}</span>
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
        <aside style={{ borderRight: 'var(--hairline)', padding: '22px 12px',
                         background: 'var(--paper-2)', display: 'flex', flexDirection: 'column', gap: 2 }}>
          <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', padding: '0 10px 8px' }}>
            This project
          </div>
          {PROJ_SECTIONS.map(s => {
            const on = sec === s.id;
            return (
              <button key={s.id} onClick={() => setSec(s.id)}
                      style={{
                        display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 10,
                        alignItems: 'center', padding: '8px 10px', borderRadius: 5,
                        textAlign: 'left',
                        background: on ? 'var(--paper)' : 'transparent',
                        color: on ? 'var(--sumi)' : 'var(--sumi-2)', fontSize: 13
                      }}>
                <span className="kanji" style={{ fontSize: 13, width: 14,
                              color: on ? 'var(--shu)' : 'var(--sumi-3)' }}>{s.kanji}</span>
                <span>{s.label}</span>
              </button>
            );
          })}
          <div style={{ height: 12 }}/>
          <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', padding: '0 10px 8px' }}>Quick</div>
          <button style={{ padding: '8px 10px', fontSize: 12, textAlign: 'left',
                            color: 'var(--sumi-2)' }}>◌ open in terminal</button>
          <button style={{ padding: '8px 10px', fontSize: 12, textAlign: 'left',
                            color: 'var(--sumi-2)' }}>◌ start session</button>
          <button style={{ padding: '8px 10px', fontSize: 12, textAlign: 'left',
                            color: 'var(--sumi-2)' }}>◌ scan now</button>
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
              <div style={{ padding: '30px 44px 0',
                             display: 'flex', alignItems: 'baseline', gap: 12,
                             borderTop: s.id !== "overview" ? 'var(--hairline)' : 'none',
                             marginTop: s.id !== "overview" ? 6 : 0 }}>
                <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>{s.kanji}</span>
                <h2 className="display" style={{ fontSize: 22, fontWeight: 400,
                              margin: 0, letterSpacing: '-0.01em' }}>{s.label}</h2>
              </div>
              {renderSection(s.id, project, openAction)}
            </section>
          ))}
          <div style={{ height: 60 }}/>
        </main>

        <aside style={{ borderLeft: 'var(--hairline)', padding: '28px 18px',
                         background: 'var(--paper-2)',
                         display: 'flex', flexDirection: 'column', gap: 2,
                         position: 'sticky', top: 0, alignSelf: 'start' }}>
          <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', padding: '0 10px 12px' }}>On this page</div>
          {PROJ_SECTIONS.map(s => {
            const on = active === s.id;
            return (
              <button key={s.id} onClick={() => goto(s.id)}
                      style={{
                        padding: '7px 10px', fontSize: 12, textAlign: 'left',
                        display: 'grid', gridTemplateColumns: 'auto 1fr',
                        gap: 10, alignItems: 'center',
                        color: on ? 'var(--sumi)' : 'var(--sumi-3)',
                        borderLeft: on ? '2px solid var(--shu)' : '2px solid transparent',
                        paddingLeft: 12
                      }}>
                <span className="kanji" style={{ fontSize: 12, width: 12,
                              color: on ? 'var(--shu)' : 'var(--sumi-4)' }}>{s.kanji}</span>
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
  ProjectSettingsV1Page, ProjectSettingsV2Page
});

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
      <div style={{ padding: '0 44px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 2, background: 'var(--paper)' }}>
        {PROJ_SECTIONS.map(s => {
          const on = s.id === "settings";
          return (
            <div key={s.id}
                 style={{ padding: '14px 18px 12px', fontSize: 12.5,
                           display: 'inline-flex', alignItems: 'center', gap: 8,
                           borderBottom: on ? '2px solid var(--shu)' : '2px solid transparent',
                           color: on ? 'var(--sumi)' : 'var(--sumi-3)',
                           marginBottom: -1 }}>
              <span className="kanji" style={{ fontSize: 12,
                            color: on ? 'var(--shu)' : 'var(--sumi-4)' }}>{s.kanji}</span>
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
      <div style={{ padding: '0 44px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 2, background: 'var(--paper)' }}>
        {PROJ_SECTIONS.map(s => {
          const on = s.id === "settings";
          return (
            <div key={s.id}
                 style={{ padding: '14px 18px 12px', fontSize: 12.5,
                           display: 'inline-flex', alignItems: 'center', gap: 8,
                           borderBottom: on ? '2px solid var(--shu)' : '2px solid transparent',
                           color: on ? 'var(--sumi)' : 'var(--sumi-3)',
                           marginBottom: -1 }}>
              <span className="kanji" style={{ fontSize: 12,
                            color: on ? 'var(--shu)' : 'var(--sumi-4)' }}>{s.kanji}</span>
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
