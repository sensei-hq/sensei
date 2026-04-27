// Projects navigation — three variations.
// (A) cards grid with filters, (B) command-K palette, (C) combined browser + grid

const { useState: nvS, useEffect: nvE, useMemo: nvM } = React;

function StatusDot({ ftr, warn }) {
  const color = warn ? 'var(--amber)' :
                ftr >= 0.8 ? 'var(--jade)' :
                ftr >= 0.6 ? 'var(--sumi-3)' : 'var(--amber)';
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
      <div style={{ padding: '28px 56px 20px',
                    display: 'flex', alignItems: 'flex-end', gap: 20, borderBottom: 'var(--hairline)' }}>
        <div className="kanji" style={{ fontSize: 46, color: 'var(--shu)', lineHeight: 1 }}>場</div>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase', marginBottom: 6 }}>Projects</div>
          <h1 className="display" style={{ fontSize: 26, fontWeight: 400, margin: 0 }}>
            All the places you work.
          </h1>
        </div>
        <button style={{ padding: '9px 14px', fontSize: 12,
                         background: 'var(--sumi)', color: 'var(--paper)', borderRadius: 5 }}>
          + new project
        </button>
      </div>

      <div style={{ padding: '14px 56px', borderBottom: 'var(--hairline)',
                    display: 'flex', gap: 18, alignItems: 'center' }}>
        <div style={{ display: 'flex', gap: 2 }}>
          {[
            ["all",      "All",       "全", counts.all],
            ["active",   "Active",    "動", counts.active],
            ["dormant",  "Dormant",   "眠", counts.dormant],
            ["archived", "Archived",  "蔵", counts.archived],
          ].map(([v, l, k, n]) => {
            const on = status === v;
            return (
              <button key={v} onClick={() => setStatus(v)}
                      style={{ padding: '6px 12px', fontSize: 11.5,
                                borderRadius: 4, display: 'inline-flex', gap: 7, alignItems: 'center',
                                background: on ? 'var(--sumi)' : 'transparent',
                                color: on ? 'var(--paper)' : 'var(--sumi-2)' }}>
                <span className="kanji" style={{ fontSize: 11 }}>{k}</span>
                {l}
                <span className="mono" style={{ fontSize: 10,
                              color: on ? 'var(--paper)' : 'var(--sumi-4)', opacity: 0.85 }}>
                  {n}
                </span>
              </button>
            );
          })}
        </div>
        <span style={{ flex: 1 }}/>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                       background: 'var(--paper-2)', borderRadius: 5,
                       padding: '6px 10px', border: 'var(--hairline)', minWidth: 260 }}>
          <span className="kanji" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>探</span>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search projects or clients…"
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 12, flex: 1, color: 'var(--sumi)' }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ fontSize: 11, color: 'var(--sumi-4)' }}>×</button>
          )}
        </div>
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
          {filtered.length} of {D.projects.length}
        </span>
      </div>

      <main style={{ flex: 1, overflow: 'auto', padding: '24px 56px 40px',
                     display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)',
                     gap: 12, alignContent: 'start' }}>
        {filtered.length === 0 && (
          <div style={{ gridColumn: '1/-1', textAlign: 'center',
                         padding: '40px 0', fontSize: 12, color: 'var(--sumi-3)' }}>
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
      padding: '12px 14px', background: 'var(--paper-2)',
      border: 'var(--hairline)', borderRadius: 8,
      opacity: p.status === "archived" ? 0.6 : 1,
      display: 'flex', flexDirection: 'column', gap: 10,
      textAlign: 'left', cursor: onOpen ? 'pointer' : 'default',
      transition: 'background 0.12s, border-color 0.12s'
    }}
    onMouseEnter={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-3)'; }}
    onMouseLeave={(e) => { if (onOpen) e.currentTarget.style.background = 'var(--paper-2)'; }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <span className="kanji" style={{ fontSize: 20, color: 'var(--shu)', lineHeight: 1,
                      width: 24, textAlign: 'center', flexShrink: 0 }}>
          {p.kanji}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', gap: 7, alignItems: 'center' }}>
            <StatusDot ftr={p.ftr} warn={p.warn}/>
            <span style={{ fontSize: 13, color: 'var(--sumi)',
                           whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {p.name}
            </span>
          </div>
          <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 1 }}>
            {p.client}
          </div>
        </div>
        {p.status !== "active" && (
          <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-3)',
                        textTransform: 'uppercase', letterSpacing: '0.12em' }}>
            {p.status === "recent" ? "dormant" : p.status}
          </span>
        )}
      </div>

      {hasStats ? (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 10,
                       paddingTop: 4, borderTop: 'var(--hairline)' }}>
          <Stat label="FTR" value={Math.round(p.ftr * 100)}
                tone={p.warn ? 'var(--amber)' : 'var(--sumi)'}/>
          <Stat label="7d" value={p.sessions7d}/>
          <Stat label="repos·libs" value={`${p.repos}·${p.libs}`}/>
        </div>
      ) : (
        <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-4)',
                      paddingTop: 4, borderTop: 'var(--hairline)',
                      display: 'flex', justifyContent: 'space-between' }}>
          <span>{p.repos} repo{p.repos !== 1 ? 's' : ''} · {p.libs} libs</span>
          <span>last · {p.lastSession}</span>
        </div>
      )}
    </button>
  );
}

function Stat({ label, value, tone = 'var(--sumi)' }) {
  return (
    <div>
      <div style={{ fontSize: 9, letterSpacing: '0.12em', color: 'var(--sumi-3)',
                    textTransform: 'uppercase' }}>{label}</div>
      <div className="display" style={{ fontSize: 16, fontWeight: 400, color: tone,
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
      <div style={{ position: 'absolute', inset: '38px 0 0', background: 'var(--paper)',
                    padding: '40px 56px', filter: 'blur(1px)', opacity: 0.7 }}>
        <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                      textTransform: 'uppercase', marginBottom: 6 }}>Wed · 22 Apr</div>
        <h1 className="display" style={{ fontSize: 28, fontWeight: 400, margin: 0 }}>
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
        <div style={{ padding: '14px 18px', borderBottom: 'var(--hairline)',
                      display: 'flex', alignItems: 'center', gap: 10 }}>
          <span className="kanji" style={{ fontSize: 16, color: 'var(--shu)' }}>探</span>
          <input value={q} onChange={(e) => setQ(e.target.value)}
                 placeholder="jump to a project, library, session…"
                 autoFocus
                 style={{ flex: 1, fontSize: 15, border: 'none', outline: 'none',
                          background: 'transparent' }}/>
          <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                        padding: '3px 7px', border: 'var(--hairline)', borderRadius: 3 }}>⌘K</span>
        </div>

        <div style={{ flex: 1, overflow: 'auto', padding: '10px 0' }}>
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
              <div style={{ padding: '10px 18px', fontSize: 11.5, color: 'var(--sumi-4)' }}>
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

        <div style={{ padding: '10px 18px', borderTop: 'var(--hairline)',
                      background: 'var(--paper-2)',
                      display: 'flex', alignItems: 'center', gap: 14,
                      fontSize: 10.5, color: 'var(--sumi-3)' }}>
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
    <div style={{ marginBottom: 6 }}>
      <div style={{ padding: '8px 18px 4px', fontSize: 9.5, letterSpacing: '0.16em',
                    color: 'var(--sumi-3)', textTransform: 'uppercase' }}>
        {label}
        <span className="mono" style={{ marginLeft: 6, color: 'var(--sumi-4)' }}>· {count}</span>
      </div>
      {children}
    </div>
  );
}

function PaletteRow({ kanji, title, sub, trail, highlight, warn }) {
  return (
    <div style={{
      display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 14,
      alignItems: 'center', padding: '9px 18px',
      background: highlight ? 'var(--paper-2)' : 'transparent'
    }}>
      <span className="kanji" style={{ fontSize: 15,
                    color: warn ? 'var(--amber)' : 'var(--shu)', width: 20 }}>
        {kanji}
      </span>
      <div>
        <div style={{ fontSize: 13, color: 'var(--sumi)' }}>{title}</div>
        <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 1 }}>{sub}</div>
      </div>
      <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>{trail}</span>
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
        <aside style={{ borderRight: 'var(--hairline)', background: 'var(--paper-2)',
                         overflow: 'auto', padding: '22px 10px' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, padding: '0 10px 18px' }}>
            <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>場</span>
            <span className="display" style={{ fontSize: 17 }}>Projects</span>
          </div>
          <TreeGroup label="Active" kanji="動" items={active} selected={selected} setSelected={setSelected}/>
          <TreeGroup label="Recent" kanji="旧" items={recent} selected={selected} setSelected={setSelected} dim/>
          <TreeGroup label="Archived" kanji="蔵" items={archived} selected={selected} setSelected={setSelected} dim/>
          <button style={{ padding: '8px 10px', fontSize: 11.5, marginTop: 12,
                            color: 'var(--shu)' }}>+ new project</button>
        </aside>

        <main style={{ overflow: 'auto', padding: '32px 48px' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 8 }}>
            <span className="kanji" style={{ fontSize: 32, color: 'var(--shu)' }}>場</span>
            <div>
              <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                             textTransform: 'uppercase' }}>Workspace</div>
              <h1 className="display" style={{ fontSize: 24, fontWeight: 400, margin: 0 }}>
                3 active · 2 dormant · 1 archived
              </h1>
            </div>
            <span style={{ flex: 1 }}/>
            <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)',
                          padding: '5px 10px', border: 'var(--hairline)', borderRadius: 3 }}>
              ⌘K to jump
            </span>
          </div>

          <h2 className="display" style={{ fontSize: 15, fontWeight: 400, margin: '28px 0 10px' }}>
            Active
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 14 }}>
            {active.map(p => <BrowserCard key={p.id} p={p} big
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>

          <h2 className="display" style={{ fontSize: 15, fontWeight: 400, margin: '28px 0 10px',
                        color: 'var(--sumi-2)' }}>
            Recent
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
            {recent.map(p => <BrowserCard key={p.id} p={p}
              selected={selected === p.id} onClick={() => setSelected(p.id)}/>)}
          </div>

          <h2 className="display" style={{ fontSize: 15, fontWeight: 400, margin: '28px 0 10px',
                        color: 'var(--sumi-3)' }}>
            Archived
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
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
    <div style={{ marginBottom: 14 }}>
      <div style={{ padding: '2px 10px 6px',
                     fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', display: 'flex', gap: 8, alignItems: 'center' }}>
        <span className="kanji" style={{ fontSize: 11 }}>{kanji}</span>
        <span>{label}</span>
        <span className="mono" style={{ marginLeft: 'auto', color: 'var(--sumi-4)' }}>
          {items.length}
        </span>
      </div>
      {items.map(p => {
        const on = selected === p.id;
        return (
          <button key={p.id} onClick={() => setSelected(p.id)}
                  style={{
                    display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 10,
                    alignItems: 'center', padding: '7px 10px', borderRadius: 5,
                    textAlign: 'left', width: '100%',
                    background: on ? 'var(--paper)' : 'transparent',
                    color: on ? 'var(--sumi)' : 'var(--sumi-2)',
                    opacity: dim ? 0.75 : 1, fontSize: 12.5
                  }}>
            <span className="kanji" style={{ fontSize: 12, width: 12,
                          color: on ? 'var(--shu)' : 'var(--sumi-3)' }}>{p.kanji}</span>
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
        border: selected ? '1px solid var(--shu)' : 'var(--hairline)',
        borderRadius: 10, textAlign: 'left',
        display: 'flex', flexDirection: 'column', gap: 10,
        opacity: dim ? 0.7 : 1,
        minHeight: big ? 120 : 84
      }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <span className="kanji" style={{ fontSize: big ? 26 : 18, color: 'var(--shu)' }}>
          {p.kanji}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
            <StatusDot ftr={p.ftr} warn={p.warn}/>
            <span style={{ fontSize: big ? 14.5 : 13, color: 'var(--sumi)' }}>{p.name}</span>
          </div>
          <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 2 }}>
            {p.client}
          </div>
        </div>
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
          {Math.round(p.ftr * 100)}
        </span>
      </div>
      {big && (
        <div style={{ display: 'flex', gap: 14, fontSize: 10.5,
                       color: 'var(--sumi-3)', marginTop: 'auto' }} className="mono">
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
