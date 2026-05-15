// Projects navigation — three variations.
// (A) cards grid with filters, (B) command-K palette, (C) combined browser + grid

const { useState: nvS, useEffect: nvE, useMemo: nvM } = React;

function StatusDot({ ftr, warn }) {
  const color = warn ? 'var(--warning)' :
                ftr >= 0.8 ? 'var(--success)' :
                ftr >= 0.6 ? 'var(--ink-3)' : 'var(--warning)';
  return <span style={{ width: 7, height: 7, borderRadius: '50%', background: color,
                 display: 'inline-block', flexShrink: 0 }}/>;
}

// ═════════════════════════════════════════════════════════════
// Variation A — Projects grid with search + status filter
// Denser cards. Dormant / archived projects render without stats.
// ═════════════════════════════════════════════════════════════
function ProjectsIndexA({ embedded = false, onOpenProject } = {}) {
  const D = window.PROJECTS_INDEX;
  const [status, setStatus] = nvS("all");   // all | active | dormant | archived
  const [query, setQuery] = nvS("");

  const counts = {
    all:      D.projects.length,
    active:   D.projects.filter(p => p.status === "active").length,
    dormant:  D.projects.filter(p => p.status === "recent").length,
    archived: D.projects.filter(p => p.status === "archived").length,
  };

  const ql = query.toLowerCase().trim();
  const filtered = D.projects.filter(p => {
    if (status === "active"   && p.status !== "active")   return false;
    if (status === "dormant"  && p.status !== "recent")   return false;
    if (status === "archived" && p.status !== "archived") return false;
    if (ql && !(p.name.toLowerCase().includes(ql) ||
                p.client.toLowerCase().includes(ql))) return false;
    return true;
  });

  // Sort: active first by FTR desc, then dormant by last session, archived last
  const order = { active: 0, recent: 1, archived: 2 };
  filtered.sort((a, b) => (order[a.status] - order[b.status]) || (b.ftr - a.ftr));

  return (
    <div className="sensei" data-screen-label="Projects · Grid"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      {!embedded && <TauriChrome title="Sensei  先生  ·  projects"/>}
      <div style={{ borderBottom: 'var(--hairline)' }} className="pt-5 pb-4 px-7" >
        <KanjiHeader variant="h1"
                     kanji="場"
                     eyebrow="Projects"
                     title="All the places you work."
                     right={
                       <button style={{ fontSize: 13, background: 'var(--ink)',
                                        color: 'var(--paper)', borderRadius: 5 }} className="py-2 px-3">
                         + new project
                       </button>
                     }/>
      </div>

      <div style={{
 borderBottom: 'var(--hairline)',
                    display: 'flex', alignItems: 'center'
}} className="py-3 px-7 gap-4" >
        <div style={{ display: 'flex' }} className="gap-1" >
          {[
            ["all",      "All",       "全", counts.all],
            ["active",   "Active",    "動", counts.active],
            ["dormant",  "Dormant",   "眠", counts.dormant],
            ["archived", "Archived",  "蔵", counts.archived],
          ].map(([v, l, k, n]) => {
            const on = status === v;
            return (
              <button key={v} onClick={() => setStatus(v)}
                      style={{
 fontSize: 11,
                                borderRadius: 4, display: 'inline-flex', alignItems: 'center',
                                background: on ? 'var(--ink)' : 'transparent',
                                color: on ? 'var(--paper)' : 'var(--ink-2)'
}} className="py-1 px-3 gap-2" >
                <span className="kanji" style={{ fontSize: 11 }}>{k}</span>
                {l}
                <span className="mono" style={{ fontSize: 11,
                              color: on ? 'var(--paper)' : 'var(--ink-4)', opacity: 0.85 }}>
                  {n}
                </span>
              </button>
            );
          })}
        </div>
        <span style={{ flex: 1 }}/>
        <div style={{
 display: 'flex', alignItems: 'center',
                       background: 'var(--paper-2)', borderRadius: 5, border: 'var(--hairline)', minWidth: 260
}} className="gap-2 py-1 px-2" >
          <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-3)' }}>探</span>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search projects or clients…"
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 13, flex: 1, color: 'var(--ink)' }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ fontSize: 11, color: 'var(--ink-4)' }}>×</button>
          )}
        </div>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {filtered.length} of {D.projects.length}
        </span>
      </div>

      <main style={{
 flex: 1, overflow: 'auto',
                     display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', alignContent: 'start'
}} className="gap-3 pt-5 pb-6 px-7" >
        {filtered.length === 0 && (
          <div style={{
 gridColumn: '1/-1', textAlign: 'center', fontSize: 13, color: 'var(--ink-3)'
}} className="py-6 px-0" >
            No projects match.
          </div>
        )}
        {filtered.map(p => <ProjectCard key={p.id} p={p} onOpen={onOpenProject}/>)}
      </main>
    </div>
  );
}

// Denser card. Stats only when this project has been touched recently;
// dormant and archived show a quieter secondary line instead.
function ProjectCard({ p, onOpen }) {
  const dormant = p.status !== "active";
  const hasStats = p.sessions7d > 0;

  return (
    <button onClick={() => onOpen && onOpen(p.id)} style={{
 background: 'var(--paper-2)',
      border: 'var(--hairline)', borderRadius: 8,
      opacity: p.status === "archived" ? 0.6 : 1,
      display: 'flex', flexDirection: 'column',
      textAlign: 'left', cursor: onOpen ? 'pointer' : 'default',
      transition: 'background 0.12s, border-color 0.12s'
}}
    onMouseEnter={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-3)'; }}
    onMouseLeave={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-2)'; }} className="py-3 px-3 gap-2" >
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)', lineHeight: 1,
                      width: 24, textAlign: 'center', flexShrink: 0 }}>
          {p.kanji}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
            <StatusDot ftr={p.ftr} warn={p.warn}/>
            <span style={{ fontSize: 13, color: 'var(--ink)',
                           whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {p.name}
            </span>
          </div>
          <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            {p.client}
          </div>
        </div>
        {p.status !== "active" && (
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                        textTransform: 'uppercase', letterSpacing: '0.12em' }}>
            {p.status === "recent" ? "dormant" : p.status}
          </span>
        )}
      </div>

      {hasStats ? (
        <div style={{
 display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', borderTop: 'var(--hairline)'
}} className="gap-2 pt-1" >
          <Stat label="FTR" value={Math.round(p.ftr * 100)}
                tone={p.warn ? 'var(--warning)' : 'var(--ink)'}/>
          <Stat label="7d" value={p.sessions7d}/>
          <Stat label="repos·libs" value={`${p.repos}·${p.libs}`}/>
        </div>
      ) : (
        <div className="mono pt-1" style={{
 fontSize: 11, color: 'var(--ink-4)', borderTop: 'var(--hairline)',
                      display: 'flex', justifyContent: 'space-between'
}}>
          <span>{p.repos} repo{p.repos !== 1 ? 's' : ''} · {p.libs} libs</span>
          <span>last · {p.lastSession}</span>
        </div>
      )}
    </button>
  );
}

function Stat({ label, value, tone = 'var(--ink)' }) {
  return (
    <div>
      <div style={{ fontSize: 11, letterSpacing: '0.12em', color: 'var(--ink-3)',
                    textTransform: 'uppercase' }}>{label}</div>
      <div className="display" style={{ fontSize: 15, fontWeight: 400, color: tone,
                    lineHeight: 1.1 }}>
        {value}
      </div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation B — Command-K palette, open over the observatory
// ═════════════════════════════════════════════════════════════
function ProjectsPaletteB() {
  const D = window.PROJECTS_INDEX;
  const libs = window.LIBRARIES_DATA.groups.flatMap(g => g.items);
  const [q, setQ] = nvS("");
  const ql = q.toLowerCase();
  const matches = nvM(() => {
    const projHits = D.projects.filter(p => !ql || p.name.toLowerCase().includes(ql) || p.client.toLowerCase().includes(ql));
    const libHits = libs.filter(l => !ql || l.name.toLowerCase().includes(ql));
    return { projHits, libHits };
  }, [q]);

  return (
    <div className="sensei" data-screen-label="Projects · Command palette"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <TauriChrome title="Sensei  先生  ·  ⌘K"/>

      {/* Dimmed observatory underneath (illustrative only) */}
      <div style={{
 position: 'absolute', inset: '38px 0 0', background: 'var(--paper)', filter: 'blur(1px)', opacity: 0.7
}} className="py-6 px-7" >
        <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                      textTransform: 'uppercase'
}} className="mb-1" >Wed · 22 Apr</div>
        <h1 className="display m-0" style={{ fontSize: 28, fontWeight: 400 }}>
          Good morning, Aiko.
        </h1>
      </div>
      <div style={{ position: 'absolute', inset: '38px 0 0',
                    background: 'oklch(0.22 0.012 50 / 0.28)' }}/>

      {/* The palette */}
      <div style={{ position: 'absolute', left: '50%', top: 110,
                    transform: 'translateX(-50%)',
                    width: 640, maxHeight: 560,
                    background: 'var(--paper)', borderRadius: 12,
                    boxShadow: '0 24px 60px rgba(0,0,0,0.3)',
                    border: 'var(--hairline)', overflow: 'hidden',
                    display: 'flex', flexDirection: 'column' }}>
        <div style={{
 borderBottom: 'var(--hairline)',
                      display: 'flex', alignItems: 'center'
}} className="py-3 px-4 gap-2" >
          <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>探</span>
          <input value={q} onChange={(e) => setQ(e.target.value)}
                 placeholder="jump to a project, library, session…"
                 autoFocus
                 style={{ flex: 1, fontSize: 15, border: 'none', outline: 'none',
                          background: 'transparent' }}/>
          <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-4)', border: 'var(--hairline)', borderRadius: 3
}}>⌘K</span>
        </div>

        <div style={{ flex: 1, overflow: 'auto' }} className="py-2 px-0" >
          <PaletteGroup label="Projects" count={matches.projHits.length}>
            {matches.projHits.map((p, i) => (
              <PaletteRow key={p.id}
                kanji={p.kanji}
                title={p.name}
                sub={`${p.client} · ${p.repos} repos · last session ${p.lastSession}`}
                trail={`${Math.round(p.ftr*100)} FTR`}
                highlight={i === 0}
                warn={p.warn}/>
            ))}
            {matches.projHits.length === 0 && (
              <div style={{ fontSize: 11, color: 'var(--ink-4)' }} className="py-2 px-4" >
                no projects match
              </div>
            )}
          </PaletteGroup>

          <PaletteGroup label="Libraries" count={matches.libHits.length}>
            {matches.libHits.slice(0, 6).map(l => (
              <PaletteRow key={l.id}
                kanji={l.icon}
                title={l.name}
                sub={`${l.source} · ${l.usage}× calls`}
                trail={`v${l.version}`}/>
            ))}
          </PaletteGroup>

          <PaletteGroup label="Recent sessions" count={4}>
            {window.SENSEI_DATA.sessions.slice(0,3).map(s => (
              <PaletteRow key={s.id}
                kanji="刻"
                title={s.title}
                sub={`${s.project} · ${s.time || s.date} · ${s.duration}`}
                trail={s.ftr ? "first-try" : `${s.corrections}×`}/>
            ))}
          </PaletteGroup>

          <PaletteGroup label="Commands" count={3}>
            <PaletteRow kanji="＋" title="Import a new project"
              sub="opens the setup wizard · step 4" trail="↵"/>
            <PaletteRow kanji="入" title="Import a library"
              sub="from URL · llms.txt · npm · crates" trail="↵"/>
            <PaletteRow kanji="掃" title="Run a full scan"
              sub="re-index imports · patterns · docs" trail="↵"/>
          </PaletteGroup>
        </div>

        <div style={{
 borderTop: 'var(--hairline)',
                      background: 'var(--paper-2)',
                      display: 'flex', alignItems: 'center',
                      fontSize: 11, color: 'var(--ink-3)'
}} className="py-2 px-4 gap-3" >
          <span><span className="mono">↑↓</span> move</span>
          <span><span className="mono">↵</span> open</span>
          <span><span className="mono">⌘↵</span> open in new tab</span>
          <span style={{ flex: 1 }}/>
          <span>esc to close</span>
        </div>
      </div>
    </div>
  );
}

function PaletteGroup({ label, count, children }) {
  return (
    <div className="mb-1" >
      <div style={{
 fontSize: 11, letterSpacing: '0.16em',
                    color: 'var(--ink-3)', textTransform: 'uppercase'
}} className="pt-2 pb-1 px-4" >
        {label}
        <span className="mono ml-1" style={{ color: 'var(--ink-4)' }}>· {count}</span>
      </div>
      {children}
    </div>
  );
}

function PaletteRow({ kanji, title, sub, trail, highlight, warn }) {
  return (
    <div style={{
      display: 'grid', gridTemplateColumns: 'auto 1fr auto',
      alignItems: 'center',
      background: highlight ? 'var(--paper-2)' : 'transparent'
}} className="gap-3 py-2 px-4" >
      <span className="kanji" style={{ fontSize: 15,
                    color: warn ? 'var(--warning)' : 'var(--accent)', width: 20 }}>
        {kanji}
      </span>
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>{title}</div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >{sub}</div>
      </div>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{trail}</span>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation C — Left tree browser + main grid
// ═════════════════════════════════════════════════════════════
function ProjectsBrowserC() {
  const D = window.PROJECTS_INDEX;
  const [selected, setSelected] = nvS("lumen-cloud");
  const active = D.projects.filter(p => p.status === "active");
  const recent = D.projects.filter(p => p.status === "recent");
  const archived = D.projects.filter(p => p.status === "archived");

  return (
    <div className="sensei" data-screen-label="Projects · Browser"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生  ·  projects · browser"/>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '260px 1fr', minHeight: 0 }}>
        <aside style={{
 borderRight: 'var(--hairline)', background: 'var(--paper-2)',
                         overflow: 'auto'
}} className="py-5 px-2" >
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 pt-0 pb-4 px-2" >
            <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>場</span>
            <span className="display" style={{ fontSize: 17 }}>Projects</span>
          </div>
          <TreeGroup label="Active" kanji="動" items={active} selected={selected} setSelected={setSelected}/>
          <TreeGroup label="Recent" kanji="旧" items={recent} selected={selected} setSelected={setSelected} dim/>
          <TreeGroup label="Archived" kanji="蔵" items={archived} selected={selected} setSelected={setSelected} dim/>
          <button style={{
 fontSize: 11,
                            color: 'var(--accent)'
}} className="py-2 px-2 mt-3" >+ new project</button>
        </aside>

        <main style={{ overflow: 'auto' }} className="py-6 px-7" >
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-3 mb-2" >
            <span className="kanji" style={{ fontSize: 28, color: 'var(--accent)' }}>場</span>
            <div>
              <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                             textTransform: 'uppercase' }}>Workspace</div>
              <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400 }}>
                3 active · 2 dormant · 1 archived
              </h1>
            </div>
            <span style={{ flex: 1 }}/>
            <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-3)', border: 'var(--hairline)', borderRadius: 3
}}>
              ⌘K to jump
            </span>
          </div>

          <h2 className="display mt-5 mb-2" style={{ fontSize: 15, fontWeight: 400 }}>
            Active
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)' }} className="gap-3" >
            {active.map(p => <BrowserCard key={p.id} p={p} big
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>

          <h2 className="display mt-5 mb-2" style={{
 fontSize: 15, fontWeight: 400,
                        color: 'var(--ink-2)'
}}>
            Recent
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3" >
            {recent.map(p => <BrowserCard key={p.id} p={p}
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>

          <h2 className="display mt-5 mb-2" style={{
 fontSize: 15, fontWeight: 400,
                        color: 'var(--ink-3)'
}}>
            Archived
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3" >
            {archived.map(p => <BrowserCard key={p.id} p={p} dim
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>
        </main>
      </div>
    </div>
  );
}

function TreeGroup({ label, kanji, items, selected, setSelected, dim }) {
  return (
    <div className="mb-3" >
      <div style={{
                     fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                     textTransform: 'uppercase', display: 'flex', alignItems: 'center'
}} className="gap-2 py-1 px-2" >
        <span className="kanji" style={{ fontSize: 11 }}>{kanji}</span>
        <span>{label}</span>
        <span className="mono ml-auto" style={{ color: 'var(--ink-4)' }}>
          {items.length}
        </span>
      </div>
      {items.map(p => {
        const on = selected === p.id;
        return (
          <button key={p.id} onClick={() => setSelected(p.id)}
                  style={{
                    display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                    alignItems: 'center', borderRadius: 5,
                    textAlign: 'left', width: '100%',
                    background: on ? 'var(--paper)' : 'transparent',
                    color: on ? 'var(--ink)' : 'var(--ink-2)',
                    opacity: dim ? 0.75 : 1, fontSize: 13
}} className="gap-2 py-2 px-2" >
            <span className="kanji" style={{ fontSize: 13, width: 12,
                          color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{p.kanji}</span>
            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {p.name}
            </span>
            <StatusDot ftr={p.ftr} warn={p.warn}/>
          </button>
        );
      })}
    </div>
  );
}

function BrowserCard({ p, big, dim, selected, onClick }) {
  return (
    <button onClick={onClick}
      style={{
        padding: big ? '18px 20px' : '14px 16px',
        background: selected ? 'var(--paper)' : 'var(--paper-2)',
        border: selected ? '1px solid var(--accent)' : 'var(--hairline)',
        borderRadius: 10, textAlign: 'left',
        display: 'flex', flexDirection: 'column',
        opacity: dim ? 0.7 : 1,
        minHeight: big ? 120 : 84
}} className="gap-2" >
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-3" >
        <span className="kanji" style={{ fontSize: big ? 26 : 18, color: 'var(--accent)' }}>
          {p.kanji}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
            <StatusDot ftr={p.ftr} warn={p.warn}/>
            <span style={{ fontSize: big ? 14.5 : 13, color: 'var(--ink)' }}>{p.name}</span>
          </div>
          <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            {p.client}
          </div>
        </div>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {Math.round(p.ftr * 100)}
        </span>
      </div>
      {big && (
        <div style={{
 display: 'flex', fontSize: 11,
                       color: 'var(--ink-3)', marginTop: 'auto'
}} className="mono gap-3">
          <span>{p.sessions7d}× 7d</span>
          <span>·</span>
          <span>{p.repos} repos</span>
          <span>·</span>
          <span>{p.libs} libs</span>
          <span>·</span>
          <span>{p.lastSession}</span>
        </div>
      )}
    </button>
  );
}

Object.assign(window, { ProjectsIndexA, ProjectsPaletteB, ProjectsBrowserC });
