// Projects navigation — three variations.
// (A) cards grid with filters, (B) command-K palette, (C) combined browser + grid

const { useState: nvS, useEffect: nvE, useMemo: nvM } = React;

function StatusDot({ ftr, warn }) {
  const color = warn ? 'var(--warning)' :
                ftr >= 0.8 ? 'var(--success)' :
                ftr >= 0.6 ? 'var(--ink-3)' : 'var(--warning)';
  return <span className="rounded-full inline-block shrink-0" style={{ width: 7, height: 7, background: color }}/>;
}

// ═════════════════════════════════════════════════════════════
// Variation A — Projects grid with search + status filter
// Denser cards. Dormant / archived projects render without stats.
// ═════════════════════════════════════════════════════════════
function ProjectsIndexA({ embedded = false, onOpenProject } = {}) {
  const D = window.PROJECTS_INDEX;
  const [status, setStatus] = nvS("all");   // all | active | dormant | archived
  const [query, setQuery] = nvS("");
  const [view, setView] = nvS("grid");       // grid | list

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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Projects · Grid"
 >
      {!embedded && <TauriChrome title="Sensei  先生  ·  projects"/>}
      <div className="pt-6 pb-4 px-12 border-b" >
        <KanjiHeader variant="h1"
                     kanji="場"
                     eyebrow="Projects"
                     title="All the places you work."
                     right={
                       <button style={{ fontSize: 13, borderRadius: 5 }} className="py-2 px-3 bg-ink text-paper">
                         + new project
                       </button>
                     }/>
      </div>

      <div className="py-3 px-12 gap-4 border-b flex items-center" >
        <div className="gap-1 flex" >
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
 borderRadius: 4,
 background: on ? 'var(--ink)' : 'transparent',
 color: on ? 'var(--paper)' : 'var(--ink-2)'
 }} className="py-1 px-3 gap-2 inline-flex items-center" >
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
        <span className="flex-1" />
        <div style={{ borderRadius: 5 }} className="gap-1 p-1 flex bg-paper-2 border border-paper-edge" >
          {[["grid", "田"], ["list", "≣"]].map(([v, g]) => (
            <button key={v} onClick={() => setView(v)}
 style={{ fontSize: 11, borderRadius: 3,
 background: view === v ? 'var(--paper)' : 'transparent',
 color: view === v ? 'var(--ink)' : 'var(--ink-3)' }}
 className="py-1 px-2 cursor-pointer border-0" >
              <span className="kanji" style={{ fontSize: 12 }}>{g}</span>
            </button>
          ))}
        </div>
        <div style={{ borderRadius: 5, minWidth: 260
 }} className="gap-2 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
          <span className="kanji text-ink-3" style={{ fontSize: 11 }}>探</span>
          <input className="border-0 bg-transparent flex-1 text-ink" value={query} onChange={e => setQuery(e.target.value)}
 placeholder="search projects or clients…"
 style={{ outline: 'none',
 fontSize: 13 }}/>
          {query && (
            <button className="text-ink-4" onClick={() => setQuery("")}
 style={{ fontSize: 11 }}>×</button>
          )}
        </div>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {filtered.length} of {D.projects.length}
        </span>
      </div>

      <main style={{
 flex: 1, overflow: 'auto',
                     display: view === "grid" ? 'grid' : 'block',
                     gridTemplateColumns: view === "grid" ? 'repeat(3, 1fr)' : undefined,
                     alignContent: 'start'
}} className={view === "grid" ? "gap-3 pt-6 pb-8 px-12" : "pt-4 pb-8 px-12"} >
        {filtered.length === 0 && (
          <div style={{
 gridColumn: '1/-1', fontSize: 13 }} className="py-8 px-0 text-center text-ink-3" >
            No projects match.
          </div>
        )}
        {view === "grid"
          ? filtered.map(p => <ProjectCard key={p.id} p={p} onOpen={onOpenProject}/>)
          : (
            <div className="border border-paper-edge overflow-hidden bg-paper-2" style={{ borderRadius: 8 }}>
              {filtered.map((p, i) => (
                <ProjectRow key={p.id} p={p} onOpen={onOpenProject}
                            last={i === filtered.length - 1}/>
              ))}
            </div>
          )}
      </main>
    </div>
  );
}

// A single project as a list row — same data as the card, laid out inline.
function ProjectRow({ p, onOpen, last }) {
  const dormant = p.status !== "active";
  const hasStats = p.sessions7d > 0;
  return (
    <button onClick={() => onOpen && onOpen(p.id)} style={{
 borderBottom: last ? 'none' : 'var(--hairline)',
 opacity: p.status === "archived" ? 0.6 : 1, gridTemplateColumns: 'auto 1fr auto auto', cursor: onOpen ? 'pointer' : 'default',
 transition: 'background 0.12s'
 }}
 onMouseEnter={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-3)'; }}
 onMouseLeave={(e) => { if (onOpen) e.currentTarget.style.background = 'transparent'; }}
 className="py-3 px-4 gap-3 w-full text-left bg-transparent border-0 grid items-center" >
      <span className="kanji text-accent shrink-0" style={{ fontSize: 18, lineHeight: 1 }}>{p.kanji}</span>
      <div className="min-w-0" >
        <div className="gap-2 flex items-center" >
          <StatusDot ftr={p.ftr} warn={p.warn}/>
          <span className="text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 13 }}>{p.name}</span>
          <ProjPill>{p.client}</ProjPill>
          {dormant && <ProjPill tone="dormant">{p.status === "recent" ? "dormant" : p.status}</ProjPill>}
        </div>
        {p.vision && (
          <div className="text-ink-3 whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 12, lineHeight: 1.35, marginTop: 3 }}>
            {p.vision}
          </div>
        )}
      </div>
      <div className="mono text-ink-3 text-right whitespace-nowrap" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>
        {hasStats
          ? <span style={{ color: p.warn ? 'var(--warning)' : 'var(--ink-2)' }}>{Math.round(p.ftr * 100)}% ftr</span>
          : <span className="text-ink-4" >last · {p.lastSession}</span>}
      </div>
      <div className="mono text-ink-4 text-right whitespace-nowrap" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"', minWidth: 78 }}>
        {p.repos} repos · {p.libs} libs
      </div>
    </button>
  );
}

// Denser card. Stats only when this project has been touched recently;
// dormant and archived show a quieter secondary line instead.
function ProjectCard({ p, onOpen }) {
  const dormant = p.status !== "active";
  const hasStats = p.sessions7d > 0;

  return (
    <button onClick={() => onOpen && onOpen(p.id)} style={{ borderRadius: 8,
 opacity: p.status === "archived" ? 0.6 : 1, cursor: onOpen ? 'pointer' : 'default',
 transition: 'background 0.12s, border-color 0.12s'
 }}
 onMouseEnter={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-3)'; }}
 onMouseLeave={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-2)'; }} className="py-3 px-3 gap-2 bg-paper-2 border border-paper-edge flex flex-col text-left" >
      {/* Row 1 — kanji + name in a single row; client/status as pills on the right */}
      <div className="gap-2 flex items-center" >
        <span className="kanji text-accent shrink-0" style={{ fontSize: 18, lineHeight: 1 }}>
          {p.kanji}
        </span>
        <StatusDot ftr={p.ftr} warn={p.warn}/>
        <span className="text-ink flex-1 min-w-0 whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 13 }}>
          {p.name}
        </span>
        <ProjPill>{p.client}</ProjPill>
        {dormant && (
          <ProjPill tone="dormant">{p.status === "recent" ? "dormant" : p.status}</ProjPill>
        )}
      </div>

      {/* Row 2 — description, full width */}
      {p.vision && (
        <div className="text-ink-2" style={{ fontSize: 12, lineHeight: 1.4,
 textWrap: 'pretty' }}>
          {p.vision}
        </div>
      )}

      {/* Stats — full width, each metric its own item */}
      {hasStats ? (
        <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-2 pt-2 grid border-t" >
          <Stat label="ftr" value={Math.round(p.ftr * 100)}
                tone={p.warn ? 'var(--warning)' : 'var(--ink)'}/>
          <Stat label="repos" value={p.repos}/>
          <Stat label="libs" value={p.libs}/>
        </div>
      ) : (
        <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-2 pt-2 grid border-t" >
          <Stat label="repos" value={p.repos} tone="var(--ink-3)"/>
          <Stat label="libs" value={p.libs} tone="var(--ink-3)"/>
          <Stat label="last session" value={p.lastSession} tone="var(--ink-3)"/>
        </div>
      )}
    </button>
  );
}

// Small pill/tag — used for the client label and dormant/archived state.
function ProjPill({ children, tone }) {
  const isDormant = tone === "dormant";
  return (
    <span className="mono uppercase bg-paper-3 border border-paper-edge whitespace-nowrap shrink-0" style={{
 fontSize: 10, letterSpacing: '0.08em',
 color: isDormant ? 'var(--ink-3)' : 'var(--ink-2)',
 borderRadius: 999, padding: '4px 8px' }}>
      {children}
    </span>
  );
}

function Stat({ label, value, tone = 'var(--ink)' }) {
  return (
    <div className="flex flex-col h-full" >
      <div className="text-ink-3 uppercase" style={{ fontSize: 10, letterSpacing: '0.08em', lineHeight: 1.25 }}>{label}</div>
      <div className="display font-normal" style={{ marginTop: 'auto', fontSize: 15,
 color: tone, lineHeight: 1.1, fontFeatureSettings: '"tnum"' }}>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden relative" data-screen-label="Projects · Command palette"
 >
      <TauriChrome title="Sensei  先生  ·  ⌘K"/>

      {/* Dimmed observatory underneath (illustrative only) */}
      <div style={{ inset: '32px 0 0', filter: 'blur(1px)', opacity: 0.7
 }} className="py-8 px-12 absolute bg-paper" >
        <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >Wed · 22 Apr</div>
        <h1 className="display m-0 font-normal" style={{ fontSize: 28 }}>
          Good morning, Aiko.
        </h1>
      </div>
      <div className="absolute" style={{ inset: '32px 0 0',
 background: 'var(--scrim)' }}/>

      {/* The palette */}
      <div className="absolute bg-paper shadow-lg border border-paper-edge overflow-hidden flex flex-col" style={{ left: '50%', top: 110,
 transform: 'translateX(-50%)',
 width: 640, maxHeight: 560, borderRadius: 12 }}>
        <div className="py-3 px-4 gap-2 border-b flex items-center" >
          <span className="kanji text-accent" style={{ fontSize: 15 }}>探</span>
          <input className="flex-1 border-0 bg-transparent" value={q} onChange={(e) => setQ(e.target.value)}
 placeholder="jump to a project, library, session…"
 autoFocus
 style={{ fontSize: 15, outline: 'none' }}/>
          <span className="mono py-1 px-2 text-ink-4 border border-paper-edge" style={{
 fontSize: 11, borderRadius: 3
 }}>⌘K</span>
        </div>

        <div className="py-2 px-0 flex-1 overflow-auto" >
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
              <div style={{ fontSize: 11 }} className="py-2 px-4 text-ink-4" >
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
 fontSize: 11 }} className="py-2 px-4 gap-3 border-t bg-paper-2 flex items-center text-ink-3" >
          <span><span className="mono">↑↓</span> move</span>
          <span><span className="mono">↵</span> open</span>
          <span><span className="mono">⌘↵</span> open in new tab</span>
          <span className="flex-1" />
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
 fontSize: 11, letterSpacing: '0.16em' }} className="pt-2 pb-1 px-4 text-ink-3 uppercase" >
        {label}
        <span className="mono ml-1 text-ink-4" >· {count}</span>
      </div>
      {children}
    </div>
  );
}

function PaletteRow({ kanji, title, sub, trail, highlight, warn }) {
  return (
    <div style={{ gridTemplateColumns: 'auto 1fr auto',
 background: highlight ? 'var(--paper-2)' : 'transparent'
 }} className="gap-3 py-2 px-4 grid items-center" >
      <span className="kanji" style={{ fontSize: 15,
                    color: warn ? 'var(--warning)' : 'var(--accent)', width: 20 }}>
        {kanji}
      </span>
      <div>
        <div className="text-ink" style={{ fontSize: 13 }}>{title}</div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{sub}</div>
      </div>
      <span className="mono text-ink-3" style={{ fontSize: 11 }}>{trail}</span>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Projects · Browser"
 >
      <TauriChrome title="Sensei  先生  ·  projects · browser"/>

      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '260px 1fr' }}>
        <aside className="py-6 px-2 border-r bg-paper-2 overflow-auto" >
          <div className="gap-2 pt-0 pb-4 px-2 flex items-baseline" >
            <span className="kanji text-accent" style={{ fontSize: 22 }}>場</span>
            <span className="display" style={{ fontSize: 17 }}>Projects</span>
          </div>
          <TreeGroup label="Active" kanji="動" items={active} selected={selected} setSelected={setSelected}/>
          <TreeGroup label="Recent" kanji="旧" items={recent} selected={selected} setSelected={setSelected} dim/>
          <TreeGroup label="Archived" kanji="蔵" items={archived} selected={selected} setSelected={setSelected} dim/>
          <button style={{
 fontSize: 11 }} className="py-2 px-2 mt-3 text-accent" >+ new project</button>
        </aside>

        <main className="py-8 px-12 overflow-auto" >
          <div className="gap-3 mb-2 flex items-baseline" >
            <span className="kanji text-accent" style={{ fontSize: 28 }}>場</span>
            <div>
              <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>Workspace</div>
              <h1 className="display m-0 font-normal" style={{ fontSize: 22 }}>
                3 active · 2 dormant · 1 archived
              </h1>
            </div>
            <span className="flex-1" />
            <span className="mono py-1 px-2 text-ink-3 border border-paper-edge" style={{
 fontSize: 11, borderRadius: 3
 }}>
              ⌘K to jump
            </span>
          </div>

          <h2 className="display mt-6 mb-2 font-normal" style={{ fontSize: 15 }}>
            Active
          </h2>
          <div style={{ gridTemplateColumns: 'repeat(2, 1fr)' }} className="gap-3 grid" >
            {active.map(p => <BrowserCard key={p.id} p={p} big
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>

          <h2 className="display mt-6 mb-2 font-normal text-ink-2" style={{
 fontSize: 15 }}>
            Recent
          </h2>
          <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 grid" >
            {recent.map(p => <BrowserCard key={p.id} p={p}
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>

          <h2 className="display mt-6 mb-2 font-normal text-ink-3" style={{
 fontSize: 15 }}>
            Archived
          </h2>
          <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 grid" >
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
 fontSize: 11, letterSpacing: '0.16em' }} className="gap-2 py-1 px-2 text-ink-3 uppercase flex items-center" >
        <span className="kanji" style={{ fontSize: 11 }}>{kanji}</span>
        <span>{label}</span>
        <span className="mono ml-auto text-ink-4" >
          {items.length}
        </span>
      </div>
      {items.map(p => {
        const on = selected === p.id;
        return (
          <button key={p.id} onClick={() => setSelected(p.id)}
 style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 5,
 background: on ? 'var(--paper)' : 'transparent',
 color: on ? 'var(--ink)' : 'var(--ink-2)',
 opacity: dim ? 0.75 : 1, fontSize: 13
 }} className="gap-2 py-2 px-2 grid items-center text-left w-full" >
            <span className="kanji" style={{ fontSize: 13, width: 12,
                          color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{p.kanji}</span>
            <span className="overflow-hidden text-ellipsis whitespace-nowrap" >
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
 borderRadius: 10,
 opacity: dim ? 0.7 : 1,
 minHeight: big ? 120 : 84
 }} className="gap-2 text-left flex flex-col" >
      <div className="gap-3 flex items-center" >
        <span className="kanji text-accent" style={{ fontSize: big ? 26 : 18 }}>
          {p.kanji}
        </span>
        <div className="flex-1 min-w-0" >
          <div className="gap-2 flex items-center" >
            <StatusDot ftr={p.ftr} warn={p.warn}/>
            <span className="text-ink" style={{ fontSize: big ? 14.5 : 13 }}>{p.name}</span>
          </div>
          <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
            {p.client}
          </div>
        </div>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {Math.round(p.ftr * 100)}
        </span>
      </div>
      {big && (
        <div style={{ fontSize: 11, marginTop: 'auto'
 }} className="mono gap-3 flex text-ink-3">
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
