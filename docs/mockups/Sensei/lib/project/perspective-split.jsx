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
  { id: "atlas",        kanji: "図", label: "Atlas",        badge: "4"  },
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
 style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 6,
 background: active ? 'var(--paper-3)' : 'transparent',
 color: active ? 'var(--ink)' : (dim ? 'var(--ink-3)' : 'var(--ink-2)'),
 fontSize: 13 }} className="gap-2 py-2 px-2 grid items-center w-full text-left cursor-pointer border-0" >
      <span className="kanji" style={{ fontSize: 13, width: 14,
                    color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
      <span>{label}</span>
      {badge != null && (
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{badge}</span>
      )}
    </button>
  );
}

function PSSectionLabel({ children }) {
  return (
    <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-0 pb-2 px-2 text-ink-3 uppercase" >
      {children}
    </div>
  );
}

// ─── Collective sidebar ─────────────────────────────────────
function CollectiveSidebar({ active = "projects", onProjectClick }) {
  const D = window.OBS_DATA;
  return (
    <aside style={{ boxSizing: 'border-box'
 }} className="py-6 px-3 gap-4 border-r bg-paper-2 flex flex-col overflow-auto h-full" >
      <div className="gap-2 px-1 flex items-baseline" >
        <span className="kanji text-accent" style={{ fontSize: 22 }}>群</span>
        <span className="display" style={{ fontSize: 15 }}>Collective</span>
      </div>

      <div>
        <div className="gap-1 flex flex-col" >
          {COLL_SIDEBAR_PRIMARY.map(s => <PSItem key={s.id} {...s} active={s.id === active}/>)}

          {/* Memories — compacted to a single item (sub-views are inline tabs) */}
          <PSItem id="memories" kanji="覚" label="Memories" badge="24"
                  active={active === "memories"}/>

          {COLL_SIDEBAR_OTHER.map(s => <PSItem key={s.id} {...s} active={s.id === active}/>)}

          {/* Instruments — compacted to a single item (sub-views are inline tabs) */}
          <PSItem id="instruments-playground" kanji="具" label="Instruments" badge="7"
                  active={active === "instruments-playground"}/>

          {COLL_SIDEBAR_BOTTOM.map(s => <PSItem key={s.id} {...s} active={s.id === active}/>)}
        </div>
      </div>

      <div>
        <div className="pt-0 pb-2 px-2 flex items-baseline justify-between" >
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>Active projects</span>
          <span className="mono text-ink-4" style={{ fontSize: 11 }}>
            {D.projects.active.length}
          </span>
        </div>
        <div className="gap-1 flex flex-col" >
          {D.projects.active.map(p => (
            <button key={p.id} onClick={() => onProjectClick && onProjectClick(p.id)}
 style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 6, fontSize: 13 }} className="gap-2 py-2 px-2 grid items-center w-full text-left bg-transparent text-ink-2 cursor-pointer border-0" >
              <span className="kanji" style={{ fontSize: 13, width: 14,
                          color: p.warn ? 'var(--warning)' : 'var(--accent)' }}>{p.kanji}</span>
              <span>{p.name}</span>
              <span style={{
 fontSize: 11, borderRadius: 3
 }} className="py-1 px-1 text-ink-4 border border-paper-edge" >↗</span>
            </button>
          ))}
        </div>
        <div style={{
 fontSize: 11, lineHeight: 1.5 }} className="pt-2 pb-0 px-2 text-ink-4 italic" >
          ↗ opens in its own window
        </div>
      </div>

      <div className="flex-1" />

      <div style={{
 fontSize: 11, lineHeight: 1.6
 }} className="pt-2 pb-0 px-2 border-t text-ink-3" >
        <span className="mono">daemon · running</span>
      </div>
    </aside>
  );
}

// ─── Project sidebar (project-scoped) ───────────────────────
function ProjectSidebar({ project, active = "overview", onSwitchProject }) {
  return (
    <aside style={{ boxSizing: 'border-box'
 }} className="py-6 px-3 gap-4 border-r bg-paper-2 flex flex-col overflow-auto h-full" >
      {/* Project identity at top — h2 header via shared component. */}
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
        <PSSectionLabel>This project</PSSectionLabel>
        <div className="gap-1 flex flex-col" >
          {PROJ_SIDEBAR_SECTIONS.map(s => (
            <PSItem key={s.id} {...s} active={s.id === active}/>
          ))}
        </div>
      </div>

      <div>
        <PSSectionLabel>Health</PSSectionLabel>
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

// ─── A faux Tauri chrome with custom title + accent stripe ──
function PerspectiveChrome({ title, accent = "var(--accent)", subtitle, onClose }) {
  return (
    <div style={{
 height: 38 }} className="px-3 bg-paper-2 border-b flex items-center shrink-0 relative" >
      <div className="gap-2 flex" >
        <span className="rounded-full bg-danger" style={{ width: 11, height: 11 }}/>
        <span className="rounded-full bg-warning" style={{ width: 11, height: 11 }}/>
        <span className="rounded-full bg-success" style={{ width: 11, height: 11 }}/>
      </div>
      <div className="gap-2 flex-1 text-center flex items-center justify-center" >
        <span className="rounded-full" style={{ width: 5, height: 5, background: accent }}/>
        <span className="text-ink" style={{ fontSize: 13, letterSpacing: '0.04em' }}>
          {title}
        </span>
        {subtitle && (
          <span className="text-ink-3" style={{ fontSize: 11 }}>· {subtitle}</span>
        )}
      </div>
      <div style={{ width: 54 }}/>
      {/* Top accent stripe to differentiate the two windows */}
      <div className="absolute" style={{ top: 0, left: 0, right: 0, height: 2,
 background: accent, opacity: 0.55 }}/>
    </div>
  );
}

// ─── A single project window (chrome + sidebar + content) ──
function ProjectWindow({ project, height = 720, accent = "var(--accent)", onSwitchProject }) {
  return (
    <div className="sensei flex flex-col bg-paper overflow-hidden shadow-lg" data-theme="light" style={{ height,
 borderRadius: 10 }}>
      <PerspectiveChrome
        title={`先生  ·  ${project.name}`}
        subtitle="project window"
        accent={accent}/>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '220px 1fr' }}>
        <ProjectSidebar project={project} active="overview" onSwitchProject={onSwitchProject}/>
        <main className="overflow-auto relative" >
          <ProjectWindowContent project={project}/>
        </main>
      </div>
    </div>
  );
}

// ─── A single collective window (chrome + sidebar + content) ──
function CollectiveWindow({ height = 720, onProjectClick, dimContent = false, accent = "var(--success)" }) {
  return (
    <div className="sensei flex flex-col bg-paper overflow-hidden shadow-lg" data-theme="light" style={{ height,
 borderRadius: 10,
 filter: dimContent ? 'saturate(0.6) brightness(0.96)' : 'none' }}>
      <PerspectiveChrome
        title="先生  ·  collective"
        subtitle="all projects"
        accent={accent}/>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '220px 1fr' }}>
        <CollectiveSidebar active="projects" onProjectClick={onProjectClick}/>
        <main className="overflow-auto" >
          <ProjectsIndexA embedded={true} onOpenProject={onProjectClick}/>
        </main>
      </div>
    </div>
  );
}

// ─── Project window content (tab-switching showcase) ────────
function ProjectWindowContent({ project }) {
  return (
    <div className="pt-8 pb-12 px-8" >
      {/* Header */}
      <div className="gap-4 mb-6 flex items-end" >
        <span className="kanji text-accent" style={{ fontSize: 56, lineHeight: 1 }}>
          {project.kanji}
        </span>
        <div className="flex-1" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Project · {project.client || "lumen-systems"}
          </div>
          <h1 className="display m-0 font-normal" style={{
 fontSize: 28,
 letterSpacing: '-0.01em'
 }}>
            {project.name}
          </h1>
        </div>
        <div className="text-right" >
          <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>FTR · 14d</div>
          <div className="gap-1 mt-1 flex items-baseline justify-end" >
            <span className="display font-normal"
 style={{ fontSize: 28, lineHeight: 1,
 color: project.warn ? 'var(--warning)' : 'var(--ink)' }}>
              {Math.round((project.ftr || 0.78) * 100)}
            </span>
            <span className="text-ink-3" style={{ fontSize: 11 }}>%</span>
          </div>
        </div>
      </div>

      {/* Hero card */}
      <div style={{ borderRadius: 10, gridTemplateColumns: 'auto 1fr'
 }} className="py-6 px-6 gap-6 mb-6 bg-paper-2 border border-paper-edge grid" >
        <div className="kanji text-accent" style={{ fontSize: 56, lineHeight: 1 }}>聴</div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-1 text-ink-3 uppercase" >
            This project · sensei speaks
          </div>
          <div className="display mb-2 font-normal text-ink" style={{
 fontSize: 22,
 letterSpacing: '-0.01em', lineHeight: 1.25 }}>
            The AI does not know your auth.
          </div>
          <p style={{ fontSize: 13, lineHeight: 1.65 }} className="m-0 text-ink-2" >
            Three sessions corrected this week — all touched refresh or device flow.
            No integration-test persona for this module yet.
          </p>
          <div className="gap-3 mt-3 flex items-center" >
            <button style={{
 fontSize: 13, borderRadius: 5 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer" >Draft a persona →</button>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              s-2891 · s-2889 · s-2886
            </span>
          </div>
        </div>
      </div>

      {/* Three quick stat blocks */}
      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-4 mb-6 grid" >
        <ProjStat label="Sessions · 7d" value={project.sessions7d || 28} sub="3 corrected"/>
        <ProjStat label="Memories" value="11" sub="2 to share · 1 to merge" tone="var(--ink)"/>
        <ProjStat label="Doc drift" value="3" sub="of 18 referenced docs" tone="var(--warning)"/>
      </div>

      {/* Sub-section preview list */}
      <div>
        <h2 className="display mt-0 mb-3 font-normal text-ink-2" style={{
 fontSize: 15 }}>
          In this project
        </h2>
        <div style={{ gridTemplateColumns: 'repeat(2, 1fr)' }} className="gap-2 grid" >
          {[
            { k: "刻", t: "Sessions",    s: "Every session in this project · what corrected, what didn't", n: 28 },
            { k: "覚", t: "Memories",    s: "What sensei has learned working here · 11 memories · 2 ready to share", n: 11 },
            { k: "巻", t: "Traceability", s: "Docs ↔ symbols · 3 drifted, 1 broken — fix-drift prompt ready", n: 4 },
            { k: "庫", t: "Libraries",    s: "openapi-3 · stripe · postgres · tailwind — used by this project", n: 5 },
            { k: "具", t: "Instruments",  s: "Project-scoped MCP tools · scoped runs only", n: 7 },
            { k: "果", t: "Impact",       s: "Did sensei's recs work here? 2 verdicts pending review",         n: 2 },
          ].map((x, i) => (
            <div key={i} style={{ borderRadius: 6, gridTemplateColumns: 'auto 1fr auto' }} className="py-3 px-3 gap-3 bg-paper-2 border border-paper-edge grid items-center" >
              <span className="kanji text-accent" style={{ fontSize: 17 }}>{x.k}</span>
              <div>
                <div className="text-ink" style={{ fontSize: 13 }}>{x.t}</div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{x.s}</div>
              </div>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>{x.n}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function ProjStat({ label, value, sub, tone = "var(--ink)" }) {
  return (
    <div style={{ borderRadius: 8
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
        {label}
      </div>
      <div className="display font-normal" style={{ fontSize: 28, color: tone, lineHeight: 1 }}>
        {value}
      </div>
      <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{sub}</div>
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
    <div data-theme="dark" style={{
 background: 'linear-gradient(135deg, var(--paper-3), var(--paper))' }} className="p-6 w-full h-full overflow-hidden flex items-center justify-center relative" >
      {/* Faint dock hint at the bottom */}
      <div style={{ bottom: 8, left: '50%', transform: 'translateX(-50%)', opacity: 0.45
 }} className="gap-1 absolute flex" >
        {Array.from({ length: 8 }).map((_, i) => (
          <div className="bg-paper-3" key={i} style={{ width: 26, height: 26, borderRadius: 6 }}/>
        ))}
      </div>

      <div style={{ gridTemplateColumns: '1fr 1fr', maxWidth: 1360
 }} className="gap-6 grid w-full" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 pl-1 text-ink-3 uppercase" >
            Window 1 · Collective perspective
          </div>
          <CollectiveWindow height={680} accent="var(--success)"
                             onProjectClick={() => {}}/>
        </div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 pl-1 text-ink-3 uppercase" >
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
    <div className="w-full h-full flex flex-col bg-paper overflow-hidden" >
      <PerspectiveChrome
        title={`先生  ·  ${project.name}`}
        subtitle="project window · own sidebar · own scope"
        accent="var(--accent)"/>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '230px 1fr' }}>
        <ProjectSidebar project={project} active="overview"/>
        <main className="overflow-auto" >
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
    <div data-theme="dark" style={{
 background: 'linear-gradient(135deg, var(--paper-3), var(--paper))', boxSizing: 'border-box'
 }} className="p-6 w-full h-full overflow-hidden relative" >
      {/* Back window — collective, slightly offset top-left, dimmed */}
      <div className="absolute" style={{ top: 28, left: 28, right: 220, bottom: 120 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 pl-1 text-ink-3 uppercase" >
          Behind · Collective (still open)
        </div>
        <CollectiveWindow height="calc(100% - 28px)" accent="var(--success)"
                           dimContent={true}/>
      </div>

      {/* Faint motion arrow from a project row to the front window */}
      <svg className="absolute w-full h-full" style={{ top: 0, left: 0,
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
      <div className="absolute" style={{ top: 90, right: 50, bottom: 50,
 width: 'calc(60% - 50px)', minWidth: 720 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 pl-1 text-ink-3 uppercase" >
          Front · Project window · just opened
        </div>
        <div className="relative" style={{ height: 'calc(100% - 28px)' }}>
          {/* Glow behind the new window */}
          <div className="absolute" style={{ inset: -8, borderRadius: 14,
 background: 'radial-gradient(circle at 50% 0%, var(--accent-edge), transparent 70%)',
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
