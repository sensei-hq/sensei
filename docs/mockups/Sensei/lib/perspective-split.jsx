// Perspective split — 3 variations for the Collective ↔ Project model.
// Clicking a project in the Collective window launches a separate Project
// window. The two perspectives have distinct sidebars and own chrome.
//
// Sections inside the Project window come from `project_contents`:
//   Overview · Sessions · Memories · Traceability · Libraries · Instruments · Patterns/Insights · Impact

const { useState: psS } = React;

// ─── Project-window sidebar (used inside every variation) ───
const PROJ_SIDEBAR_SECTIONS = [
  { id: "overview",     kanji: "全", label: "Overview"    },
  { id: "sessions",     kanji: "刻", label: "Sessions",     badge: "28" },
  { id: "memories",     kanji: "覚", label: "Memories",     badge: "11" },
  { id: "traceability", kanji: "巻", label: "Traceability", badge: "4"  },
  { id: "libraries",    kanji: "庫", label: "Libraries",    badge: "5"  },
  { id: "instruments",  kanji: "具", label: "Instruments",  badge: "7"  },
  { id: "patterns",     kanji: "紋", label: "Patterns",     badge: "3"  },
  { id: "impact",       kanji: "果", label: "Impact",       badge: "2"  },
  { id: "settings",     kanji: "設", label: "Settings"      },
];

// Collective sidebar (mirrors the answers from collective_contents)
const COLL_SIDEBAR_PRIMARY = [
  { id: "today",       kanji: "家", label: "Today" },
  { id: "projects",    kanji: "場", label: "Projects",    badge: "5" },
  { id: "sessions",    kanji: "録", label: "Sessions",    badge: "41" },
  { id: "insights",    kanji: "今", label: "Insights",    badge: "6" },
];

const COLL_SIDEBAR_MEMORIES = [
  { id: "memories",      kanji: "解", label: "Anatomy" },
  { id: "share-review",  kanji: "共", label: "Sharing",      badge: "4" },
  { id: "consolidation", kanji: "結", label: "Consolidate",  badge: "3" },
];

const COLL_SIDEBAR_INSTRUMENTS = [
  { id: "instruments-playground", kanji: "試", label: "Playground" },
  { id: "instruments-replay",     kanji: "録", label: "Replay" },
  { id: "instruments-health",     kanji: "健", label: "Health" },
];

const COLL_SIDEBAR_OTHER = [
  { id: "upgrades",  kanji: "贈", label: "Upgrades", badge: "5" },
  { id: "impact",    kanji: "果", label: "Impact",   badge: "3" },
  { id: "libraries", kanji: "庫", label: "Libraries", badge: "14" },
];

const COLL_SIDEBAR_BOTTOM = [
  { id: "collective",  kanji: "群", label: "Collective intel" },
  { id: "configure",   kanji: "調", label: "Configure" },
];

// ─── A simple sidebar item ──────────────────────────────────
function PSItem({ id, kanji, label, badge, active, onClick, dim }) {
  return (
    <button onClick={onClick}
            style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto',
              alignItems: 'center', width: '100%', borderRadius: 6, textAlign: 'left',
              background: active ? 'var(--paper-3)' : 'transparent',
              color: active ? 'var(--ink)' : (dim ? 'var(--ink-3)' : 'var(--ink-2)'),
              fontSize: 13, cursor: 'pointer', border: 'none'
}} className="gap-2 py-2 px-2" >
      <span className="kanji" style={{ fontSize: 13, width: 14,
                    color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
      <span>{label}</span>
      {badge != null && (
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{badge}</span>
      )}
    </button>
  );
}

function PSSectionLabel({ children }) {
  return (
    <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                   textTransform: 'uppercase'
}} className="pt-0 pb-2 px-2" >
      {children}
    </div>
  );
}

// ─── Collective sidebar ─────────────────────────────────────
function CollectiveSidebar({ active = "projects", onProjectClick }) {
  const D = window.OBS_DATA;
  return (
    <aside style={{
 borderRight: 'var(--hairline)',
                     background: 'var(--paper-2)',
                     display: 'flex', flexDirection: 'column',
                     overflow: 'auto', height: '100%', boxSizing: 'border-box'
}} className="py-5 px-3 gap-4" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 px-1" >
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>群</span>
        <span className="display" style={{ fontSize: 15 }}>Collective</span>
      </div>

      <div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {COLL_SIDEBAR_PRIMARY.map(s => <PSItem key={s.id} {...s} active={s.id === active}/>)}
        </div>

        {/* Memories group */}
        <div className="py-1 px-2" >
          <div style={{
 display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                         alignItems: 'center',
                         color: 'var(--ink-2)', fontSize: 13
}} className="gap-2 py-2 px-0" >
            <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: 'var(--ink-3)' }}>覚</span>
            <span>Memories</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>24</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1 pl-3" >
            {COLL_SIDEBAR_MEMORIES.map(s => <PSItem key={s.id} {...s}/>)}
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {COLL_SIDEBAR_OTHER.map(s => <PSItem key={s.id} {...s}/>)}
        </div>

        {/* Instruments group */}
        <div className="py-1 px-2" >
          <div style={{
 display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                         alignItems: 'center',
                         color: 'var(--ink-2)', fontSize: 13
}} className="gap-2 py-2 px-0" >
            <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: 'var(--ink-3)' }}>具</span>
            <span>Instruments</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>7</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1 pl-3" >
            {COLL_SIDEBAR_INSTRUMENTS.map(s => <PSItem key={s.id} {...s}/>)}
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {COLL_SIDEBAR_BOTTOM.map(s => <PSItem key={s.id} {...s}/>)}
        </div>
      </div>

      <div>
        <div style={{
 display: 'flex', alignItems: 'baseline', justifyContent: 'space-between'
}} className="pt-0 pb-2 px-2" >
          <span style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                          textTransform: 'uppercase' }}>Active projects</span>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            {D.projects.active.length}
          </span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {D.projects.active.map(p => (
            <button key={p.id} onClick={() => onProjectClick && onProjectClick(p.id)}
                    style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto',
              alignItems: 'center', width: '100%', borderRadius: 6, textAlign: 'left',
              background: 'transparent',
              color: 'var(--ink-2)', fontSize: 13, cursor: 'pointer', border: 'none'
}} className="gap-2 py-2 px-2" >
              <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: p.warn ? 'var(--warning)' : 'var(--accent)' }}>{p.kanji}</span>
              <span>{p.name}</span>
              <span style={{
 fontSize: 11, color: 'var(--ink-4)', border: 'var(--hairline)', borderRadius: 3
}} className="py-1 px-1" >↗</span>
            </button>
          ))}
        </div>
        <div style={{
 fontSize: 11, color: 'var(--ink-4)', lineHeight: 1.5, fontStyle: 'italic'
}} className="pt-2 pb-0 px-2" >
          ↗ opens in its own window
        </div>
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{
 borderTop: 'var(--hairline)',
                     fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.6
}} className="pt-2 pb-0 px-2" >
        <span className="mono">daemon · running</span>
      </div>
    </aside>
  );
}

// ─── Project sidebar (project-scoped) ───────────────────────
function ProjectSidebar({ project, active = "overview", onSwitchProject }) {
  return (
    <aside style={{
 borderRight: 'var(--hairline)',
                     background: 'var(--paper-2)',
                     display: 'flex', flexDirection: 'column',
                     overflow: 'auto', height: '100%', boxSizing: 'border-box'
}} className="py-5 px-3 gap-4" >
      {/* Project identity at top — h2 header via shared component. */}
      <div className="px-1" >
        <KanjiHeader variant="h2" kanji={project.kanji} eyebrow="Project" title={project.name}/>
        <div className="mono mt-2" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {project.client || "lumen-systems"}
        </div>
        <button onClick={onSwitchProject}
                style={{
 fontSize: 11, color: 'var(--ink-3)', border: 'var(--hairline)', borderRadius: 4,
                          background: 'transparent', cursor: 'pointer'
}} className="mt-2 py-1 px-2" >
          ⇆ switch project
        </button>
      </div>

      <div>
        <PSSectionLabel>This project</PSSectionLabel>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {PROJ_SIDEBAR_SECTIONS.map(s => (
            <PSItem key={s.id} {...s} active={s.id === active}/>
          ))}
        </div>
      </div>

      <div>
        <PSSectionLabel>Health</PSSectionLabel>
        <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                       display: 'flex', flexDirection: 'column'
}} className="gap-1 px-2" >
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

      <div style={{
 borderTop: 'var(--hairline)',
                     fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.6
}} className="pt-2 pb-0 px-2" >
        <span className="mono">scoped to this project</span>
      </div>
    </aside>
  );
}

// ─── A faux Tauri chrome with custom title + accent stripe ──
function PerspectiveChrome({ title, accent = "var(--accent)", subtitle, onClose }) {
  return (
    <div style={{
      height: 38, background: 'var(--paper-2)',
      borderBottom: 'var(--hairline)',
      display: 'flex', alignItems: 'center',
      flexShrink: 0, position: 'relative'
}} className="px-3" >
      <div style={{ display: 'flex' }} className="gap-2" >
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#ed6a5e' }}/>
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#f5bf4f' }}/>
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#62c554' }}/>
      </div>
      <div style={{
 flex: 1, textAlign: 'center', display: 'flex',
                     alignItems: 'center', justifyContent: 'center'
}} className="gap-2" >
        <span style={{ width: 5, height: 5, borderRadius: '50%', background: accent }}/>
        <span style={{ fontSize: 13, color: 'var(--ink)', letterSpacing: '0.04em' }}>
          {title}
        </span>
        {subtitle && (
          <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>· {subtitle}</span>
        )}
      </div>
      <div style={{ width: 54 }}/>
      {/* Top accent stripe to differentiate the two windows */}
      <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 2,
                     background: accent, opacity: 0.55 }}/>
    </div>
  );
}

// ─── A single project window (chrome + sidebar + content) ──
function ProjectWindow({ project, height = 720, accent = "var(--accent)", onSwitchProject }) {
  return (
    <div className="sensei" style={{ height, display: 'flex', flexDirection: 'column',
                   background: 'var(--paper)', overflow: 'hidden',
                   borderRadius: 10,
                   boxShadow: '0 1px 3px rgba(0,0,0,0.08), 0 14px 40px rgba(40,30,20,0.16)' }}>
      <PerspectiveChrome
        title={`先生  ·  ${project.name}`}
        subtitle="project window"
        accent={accent}/>
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '220px 1fr', minHeight: 0 }}>
        <ProjectSidebar project={project} active="overview" onSwitchProject={onSwitchProject}/>
        <main style={{ overflow: 'auto', position: 'relative' }}>
          <ProjectWindowContent project={project}/>
        </main>
      </div>
    </div>
  );
}

// ─── A single collective window (chrome + sidebar + content) ──
function CollectiveWindow({ height = 720, onProjectClick, dimContent = false, accent = "var(--success)" }) {
  return (
    <div className="sensei" style={{ height, display: 'flex', flexDirection: 'column',
                   background: 'var(--paper)', overflow: 'hidden',
                   borderRadius: 10,
                   boxShadow: '0 1px 3px rgba(0,0,0,0.08), 0 14px 40px rgba(40,30,20,0.12)',
                   filter: dimContent ? 'saturate(0.6) brightness(0.96)' : 'none' }}>
      <PerspectiveChrome
        title="先生  ·  collective"
        subtitle="all projects"
        accent={accent}/>
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '220px 1fr', minHeight: 0 }}>
        <CollectiveSidebar active="projects" onProjectClick={onProjectClick}/>
        <main style={{ overflow: 'auto' }}>
          <ProjectsIndexA embedded={true} onOpenProject={onProjectClick}/>
        </main>
      </div>
    </div>
  );
}

// ─── Project window content (tab-switching showcase) ────────
function ProjectWindowContent({ project }) {
  return (
    <div className="pt-6 pb-7 px-6" >
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'flex-end' }} className="gap-4 mb-5" >
        <span className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>
          {project.kanji}
        </span>
        <div style={{ flex: 1 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Project · {project.client || "lumen-systems"}
          </div>
          <h1 className="display m-0" style={{
 fontSize: 28, fontWeight: 400,
                        letterSpacing: '-0.01em'
}}>
            {project.name}
          </h1>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>FTR · 14d</div>
          <div style={{
 display: 'flex', alignItems: 'baseline',
                         justifyContent: 'flex-end'
}} className="gap-1 mt-1" >
            <span className="display"
                   style={{ fontSize: 28, fontWeight: 400, lineHeight: 1,
                             color: project.warn ? 'var(--warning)' : 'var(--ink)' }}>
              {Math.round((project.ftr || 0.78) * 100)}
            </span>
            <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>%</span>
          </div>
        </div>
      </div>

      {/* Hero card */}
      <div style={{
        background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 10,
        display: 'grid', gridTemplateColumns: 'auto 1fr'
}} className="py-5 px-5 gap-5 mb-5" >
        <div className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>聴</div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            This project · sensei speaks
          </div>
          <div className="display mb-2" style={{
 fontSize: 22, fontWeight: 400,
                        letterSpacing: '-0.01em', lineHeight: 1.25,
                        color: 'var(--ink)'
}}>
            The AI does not know your auth.
          </div>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.65 }} className="m-0" >
            Three sessions corrected this week — all touched refresh or device flow.
            No integration-test persona for this module yet.
          </p>
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-3 mt-3" >
            <button style={{
 fontSize: 13, background: 'var(--ink)',
              color: 'var(--paper)', borderRadius: 5, border: 'none', cursor: 'pointer'
}} className="py-2 px-3" >Draft a persona →</button>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              s-2891 · s-2889 · s-2886
            </span>
          </div>
        </div>
      </div>

      {/* Three quick stat blocks */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-4 mb-5" >
        <ProjStat label="Sessions · 7d" value={project.sessions7d || 28} sub="3 corrected"/>
        <ProjStat label="Memories" value="11" sub="2 to share · 1 to merge" tone="var(--ink)"/>
        <ProjStat label="Doc drift" value="3" sub="of 18 referenced docs" tone="var(--warning)"/>
      </div>

      {/* Sub-section preview list */}
      <div>
        <h2 className="display mt-0 mb-3" style={{
 fontSize: 15, fontWeight: 400,
                      color: 'var(--ink-2)'
}}>
          In this project
        </h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)' }} className="gap-2" >
          {[
            { k: "刻", t: "Sessions",    s: "Every session in this project · what corrected, what didn't", n: 28 },
            { k: "覚", t: "Memories",    s: "What sensei has learned working here · 11 memories · 2 ready to share", n: 11 },
            { k: "巻", t: "Traceability", s: "Docs ↔ symbols · 3 drifted, 1 broken — fix-drift prompt ready", n: 4 },
            { k: "庫", t: "Libraries",    s: "openapi-3 · stripe · postgres · tailwind — used by this project", n: 5 },
            { k: "具", t: "Instruments",  s: "Project-scoped MCP tools · scoped runs only", n: 7 },
            { k: "果", t: "Impact",       s: "Did sensei's recs work here? 2 verdicts pending review",         n: 2 },
          ].map((x, i) => (
            <div key={i} style={{
 background: 'var(--paper-2)',
              border: 'var(--hairline)', borderRadius: 6,
              display: 'grid', gridTemplateColumns: 'auto 1fr auto',
              alignItems: 'center'
}} className="py-3 px-3 gap-3" >
              <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)' }}>{x.k}</span>
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{x.t}</div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >{x.s}</div>
              </div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{x.n}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function ProjStat({ label, value, sub, tone = "var(--ink)" }) {
  return (
    <div style={{
 background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 8
}} className="py-3 px-4" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase'
}} className="mb-1" >
        {label}
      </div>
      <div className="display" style={{ fontSize: 28, fontWeight: 400, color: tone, lineHeight: 1 }}>
        {value}
      </div>
      <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >{sub}</div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation A — Two windows side by side, hovering over desktop
// "After clicking lumen-cloud, both windows are open."
// ═════════════════════════════════════════════════════════════
function PerspectiveSplitA() {
  const D = window.OBS_DATA;
  const project = D.projects.active.find(p => p.id === "lumen-cloud") || D.projects.active[0];

  return (
    <div style={{
      width: '100%', height: '100%', overflow: 'hidden',
      background: 'linear-gradient(135deg, oklch(0.42 0.02 50), oklch(0.30 0.012 50))',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      position: 'relative'
}} className="p-5" >
      {/* Faint dock hint at the bottom */}
      <div style={{
 position: 'absolute', bottom: 8, left: '50%', transform: 'translateX(-50%)',
                     display: 'flex', opacity: 0.45
}} className="gap-1" >
        {Array.from({ length: 8 }).map((_, i) => (
          <div key={i} style={{ width: 26, height: 26, borderRadius: 6,
                                  background: 'rgba(255,255,255,0.15)' }}/>
        ))}
      </div>

      <div style={{
 display: 'grid', gridTemplateColumns: '1fr 1fr',
                     width: '100%', maxWidth: 1360
}} className="gap-5" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.55)',
                         textTransform: 'uppercase'
}} className="mb-2 pl-1" >
            Window 1 · Collective perspective
          </div>
          <CollectiveWindow height={680} accent="var(--success)"
                             onProjectClick={() => {}}/>
        </div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.55)',
                         textTransform: 'uppercase'
}} className="mb-2 pl-1" >
            Window 2 · Project perspective · {project.name}
          </div>
          <ProjectWindow project={project} height={680} accent="var(--accent)"/>
        </div>
      </div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation B — Project window standalone (full-bleed)
// "Once opened, this is what living inside a project looks like."
// ═════════════════════════════════════════════════════════════
function PerspectiveSplitB() {
  const D = window.OBS_DATA;
  const project = D.projects.active.find(p => p.id === "lumen-cloud") || D.projects.active[0];

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                   background: 'var(--paper)', overflow: 'hidden' }}>
      <PerspectiveChrome
        title={`先生  ·  ${project.name}`}
        subtitle="project window · own sidebar · own scope"
        accent="var(--accent)"/>
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '230px 1fr', minHeight: 0 }}>
        <ProjectSidebar project={project} active="overview"/>
        <main style={{ overflow: 'auto' }}>
          <ProjectWindowContent project={project}/>
        </main>
      </div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation C — Cascaded: Project window springs out over the
// dimmed Collective window (the moment of launch).
// ═════════════════════════════════════════════════════════════
function PerspectiveSplitC() {
  const D = window.OBS_DATA;
  const project = D.projects.active.find(p => p.id === "lumen-cloud") || D.projects.active[0];

  return (
    <div style={{
      width: '100%', height: '100%', overflow: 'hidden',
      background: 'linear-gradient(135deg, oklch(0.40 0.02 50), oklch(0.28 0.012 50))',
      position: 'relative', boxSizing: 'border-box'
}} className="p-5" >
      {/* Back window — collective, slightly offset top-left, dimmed */}
      <div style={{ position: 'absolute', top: 28, left: 28, right: 220, bottom: 120 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.5)',
                       textTransform: 'uppercase'
}} className="mb-2 pl-1" >
          Behind · Collective (still open)
        </div>
        <CollectiveWindow height="calc(100% - 28px)" accent="var(--success)"
                           dimContent={true}/>
      </div>

      {/* Faint motion arrow from a project row to the front window */}
      <svg style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%',
                     pointerEvents: 'none', zIndex: 5 }}>
        <defs>
          <marker id="psarrow" markerWidth="10" markerHeight="10" refX="6" refY="5"
                  orient="auto">
            <path d="M0,0 L10,5 L0,10 z" fill="var(--accent)" opacity="0.7"/>
          </marker>
        </defs>
        <path d="M 350,420 C 480,460 600,480 720,440"
              stroke="var(--accent)" strokeWidth="1.4" strokeDasharray="4 4"
              fill="none" opacity="0.7" markerEnd="url(#psarrow)"/>
      </svg>

      {/* Front window — project, springs forward */}
      <div style={{ position: 'absolute', top: 90, right: 50, bottom: 50,
                     width: 'calc(60% - 50px)', minWidth: 720 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.65)',
                       textTransform: 'uppercase'
}} className="mb-2 pl-1" >
          Front · Project window · just opened
        </div>
        <div style={{ position: 'relative', height: 'calc(100% - 28px)' }}>
          {/* Glow behind the new window */}
          <div style={{ position: 'absolute', inset: -8, borderRadius: 14,
                          background: 'radial-gradient(circle at 50% 0%, rgba(192,71,45,0.45), transparent 70%)',
                          filter: 'blur(18px)', pointerEvents: 'none' }}/>
          <ProjectWindow project={project} height="100%" accent="var(--accent)"/>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  PerspectiveSplitA,
  PerspectiveSplitB,
  PerspectiveSplitC,
  CollectiveSidebar,
  ProjectSidebar,
});
