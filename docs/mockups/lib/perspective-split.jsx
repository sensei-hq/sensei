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
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 8,
              alignItems: 'center', width: '100%',
              padding: '8px 8px', borderRadius: 6, textAlign: 'left',
              background: active ? 'var(--paper-3)' : 'transparent',
              color: active ? 'var(--ink)' : (dim ? 'var(--ink-3)' : 'var(--ink-2)'),
              fontSize: 13, cursor: 'pointer', border: 'none'
            }}>
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
    <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                   textTransform: 'uppercase', padding: '0 8px 8px' }}>
      {children}
    </div>
  );
}

// ─── Collective sidebar ─────────────────────────────────────
function CollectiveSidebar({ active = "projects", onProjectClick }) {
  const D = window.OBS_DATA;
  return (
    <aside style={{ borderRight: 'var(--hairline)', padding: '24px 12px',
                     background: 'var(--paper-2)',
                     display: 'flex', flexDirection: 'column', gap: 16,
                     overflow: 'auto', height: '100%', boxSizing: 'border-box' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, padding: '0 4px' }}>
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>群</span>
        <span className="display" style={{ fontSize: 15 }}>Collective</span>
      </div>

      <div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {COLL_SIDEBAR_PRIMARY.map(s => <PSItem key={s.id} {...s} active={s.id === active}/>)}
        </div>

        {/* Memories group */}
        <div style={{ padding: '4px 8px 4px' }}>
          <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 8,
                         alignItems: 'center', padding: '8px 0',
                         color: 'var(--ink-2)', fontSize: 13 }}>
            <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: 'var(--ink-3)' }}>覚</span>
            <span>Memories</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>24</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, paddingLeft: 12 }}>
            {COLL_SIDEBAR_MEMORIES.map(s => <PSItem key={s.id} {...s}/>)}
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {COLL_SIDEBAR_OTHER.map(s => <PSItem key={s.id} {...s}/>)}
        </div>

        {/* Instruments group */}
        <div style={{ padding: '4px 8px 4px' }}>
          <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 8,
                         alignItems: 'center', padding: '8px 0',
                         color: 'var(--ink-2)', fontSize: 13 }}>
            <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: 'var(--ink-3)' }}>具</span>
            <span>Instruments</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>7</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, paddingLeft: 12 }}>
            {COLL_SIDEBAR_INSTRUMENTS.map(s => <PSItem key={s.id} {...s}/>)}
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {COLL_SIDEBAR_BOTTOM.map(s => <PSItem key={s.id} {...s}/>)}
        </div>
      </div>

      <div>
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                       padding: '0 8px 8px' }}>
          <span style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                          textTransform: 'uppercase' }}>Active projects</span>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            {D.projects.active.length}
          </span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {D.projects.active.map(p => (
            <button key={p.id} onClick={() => onProjectClick && onProjectClick(p.id)}
                    style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 8,
              alignItems: 'center', width: '100%',
              padding: '8px 8px', borderRadius: 6, textAlign: 'left',
              background: 'transparent',
              color: 'var(--ink-2)', fontSize: 13, cursor: 'pointer', border: 'none'
            }}>
              <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: p.warn ? 'var(--warning)' : 'var(--accent)' }}>{p.kanji}</span>
              <span>{p.name}</span>
              <span style={{ fontSize: 11, color: 'var(--ink-4)',
                              padding: '4px 4px', border: 'var(--hairline)', borderRadius: 3 }}>↗</span>
            </button>
          ))}
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-4)',
                       padding: '8px 8px 0', lineHeight: 1.5, fontStyle: 'italic' }}>
          ↗ opens in its own window
        </div>
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ padding: '8px 8px 0', borderTop: 'var(--hairline)',
                     fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.6 }}>
        <span className="mono">daemon · running</span>
      </div>
    </aside>
  );
}

// ─── Project sidebar (project-scoped) ───────────────────────
function ProjectSidebar({ project, active = "overview", onSwitchProject }) {
  return (
    <aside style={{ borderRight: 'var(--hairline)', padding: '24px 12px',
                     background: 'var(--paper-2)',
                     display: 'flex', flexDirection: 'column', gap: 16,
                     overflow: 'auto', height: '100%', boxSizing: 'border-box' }}>
      {/* Project identity at top */}
      <div style={{ padding: '0 4px' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>
          Project
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span className="kanji" style={{ fontSize: 28, color: 'var(--accent)', lineHeight: 1 }}>
            {project.kanji}
          </span>
          <div style={{ minWidth: 0 }}>
            <div className="display" style={{ fontSize: 15, color: 'var(--ink)',
                          letterSpacing: '-0.01em', lineHeight: 1.1 }}>
              {project.name}
            </div>
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
        <PSSectionLabel>This project</PSSectionLabel>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {PROJ_SIDEBAR_SECTIONS.map(s => (
            <PSItem key={s.id} {...s} active={s.id === active}/>
          ))}
        </div>
      </div>

      <div>
        <PSSectionLabel>Health</PSSectionLabel>
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

// ─── A faux Tauri chrome with custom title + accent stripe ──
function PerspectiveChrome({ title, accent = "var(--accent)", subtitle, onClose }) {
  return (
    <div style={{
      height: 38, background: 'var(--paper-2)',
      borderBottom: 'var(--hairline)',
      display: 'flex', alignItems: 'center', padding: '0 12px',
      flexShrink: 0, position: 'relative'
    }}>
      <div style={{ display: 'flex', gap: 8 }}>
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#ed6a5e' }}/>
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#f5bf4f' }}/>
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#62c554' }}/>
      </div>
      <div style={{ flex: 1, textAlign: 'center', display: 'flex',
                     alignItems: 'center', justifyContent: 'center', gap: 8 }}>
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
    <div style={{ height, display: 'flex', flexDirection: 'column',
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
    <div style={{ height, display: 'flex', flexDirection: 'column',
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
    <div style={{ padding: '32px 32px 48px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'flex-end', gap: 16, marginBottom: 24 }}>
        <span className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>
          {project.kanji}
        </span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            Project · {project.client || "lumen-systems"}
          </div>
          <h1 className="display" style={{ fontSize: 28, fontWeight: 400, margin: 0,
                        letterSpacing: '-0.01em' }}>
            {project.name}
          </h1>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>FTR · 14d</div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 4,
                         justifyContent: 'flex-end', marginTop: 4 }}>
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
        padding: '24px 24px',
        background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 10,
        display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 24, marginBottom: 24
      }}>
        <div className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>聴</div>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            This project · sensei speaks
          </div>
          <div className="display" style={{ fontSize: 22, fontWeight: 400,
                        letterSpacing: '-0.01em', lineHeight: 1.25, marginBottom: 8,
                        color: 'var(--ink)' }}>
            The AI does not know your auth.
          </div>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.65, margin: 0 }}>
            Three sessions corrected this week — all touched refresh or device flow.
            No integration-test persona for this module yet.
          </p>
          <div style={{ display: 'flex', gap: 12, alignItems: 'center', marginTop: 12 }}>
            <button style={{
              padding: '8px 12px', fontSize: 13, background: 'var(--ink)',
              color: 'var(--paper)', borderRadius: 5, border: 'none', cursor: 'pointer'
            }}>Draft a persona →</button>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              s-2891 · s-2889 · s-2886
            </span>
          </div>
        </div>
      </div>

      {/* Three quick stat blocks */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16, marginBottom: 24 }}>
        <ProjStat label="Sessions · 7d" value={project.sessions7d || 28} sub="3 corrected"/>
        <ProjStat label="Memories" value="11" sub="2 to share · 1 to merge" tone="var(--ink)"/>
        <ProjStat label="Doc drift" value="3" sub="of 18 referenced docs" tone="var(--warning)"/>
      </div>

      {/* Sub-section preview list */}
      <div>
        <h2 className="display" style={{ fontSize: 15, fontWeight: 400, margin: '0 0 12px',
                      color: 'var(--ink-2)' }}>
          In this project
        </h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 8 }}>
          {[
            { k: "刻", t: "Sessions",    s: "Every session in this project · what corrected, what didn't", n: 28 },
            { k: "覚", t: "Memories",    s: "What sensei has learned working here · 11 memories · 2 ready to share", n: 11 },
            { k: "巻", t: "Traceability", s: "Docs ↔ symbols · 3 drifted, 1 broken — fix-drift prompt ready", n: 4 },
            { k: "庫", t: "Libraries",    s: "openapi-3 · stripe · postgres · tailwind — used by this project", n: 5 },
            { k: "具", t: "Instruments",  s: "Project-scoped MCP tools · scoped runs only", n: 7 },
            { k: "果", t: "Impact",       s: "Did sensei's recs work here? 2 verdicts pending review",         n: 2 },
          ].map((x, i) => (
            <div key={i} style={{
              padding: '12px 12px', background: 'var(--paper-2)',
              border: 'var(--hairline)', borderRadius: 6,
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 12,
              alignItems: 'center'
            }}>
              <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)' }}>{x.k}</span>
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{x.t}</div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>{x.s}</div>
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
    <div style={{ padding: '12px 16px', background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 8 }}>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase', marginBottom: 4 }}>
        {label}
      </div>
      <div className="display" style={{ fontSize: 28, fontWeight: 400, color: tone, lineHeight: 1 }}>
        {value}
      </div>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>{sub}</div>
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
      display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 24,
      position: 'relative'
    }}>
      {/* Faint dock hint at the bottom */}
      <div style={{ position: 'absolute', bottom: 8, left: '50%', transform: 'translateX(-50%)',
                     display: 'flex', gap: 4, opacity: 0.45 }}>
        {Array.from({ length: 8 }).map((_, i) => (
          <div key={i} style={{ width: 26, height: 26, borderRadius: 6,
                                  background: 'rgba(255,255,255,0.15)' }}/>
        ))}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24,
                     width: '100%', maxWidth: 1360 }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.55)',
                         textTransform: 'uppercase', marginBottom: 8, paddingLeft: 4 }}>
            Window 1 · Collective perspective
          </div>
          <CollectiveWindow height={680} accent="var(--success)"
                             onProjectClick={() => {}}/>
        </div>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.55)',
                         textTransform: 'uppercase', marginBottom: 8, paddingLeft: 4 }}>
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
      position: 'relative', padding: 24, boxSizing: 'border-box'
    }}>
      {/* Back window — collective, slightly offset top-left, dimmed */}
      <div style={{ position: 'absolute', top: 28, left: 28, right: 220, bottom: 120 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.5)',
                       textTransform: 'uppercase', marginBottom: 8, paddingLeft: 4 }}>
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
        <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'rgba(255,255,255,0.65)',
                       textTransform: 'uppercase', marginBottom: 8, paddingLeft: 4 }}>
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
