// Observatory · left rail
// Sections (Today, Projects, Sessions, Insights, Memories, Instruments, Libraries)
// + Active projects list + Dormant.

const NAV = [
  { id: "home",        label: "Today",       kanji: "今" },
  { id: "projects",    label: "Projects",    kanji: "場" },
  { id: "sessions",    label: "Sessions",    kanji: "刻" },
  { id: "insights",    label: "Insights",    kanji: "察" },
  { id: "memories",    label: "Memories",    kanji: "覚", badge: 24 },
  { id: "instruments", label: "Instruments", kanji: "具", badge: 7 },
  { id: "libraries",   label: "Libraries",   kanji: "庫" },
  { id: "config",      label: "Configure",   kanji: "設" },
];

const ACTIVE = [
  { id: "lumen-studio", kanji: "工", name: "Lumen Studio", ftr: 82 },
  { id: "lumen-cloud",  kanji: "雲", name: "Lumen Cloud",  ftr: 64, warn: true },
  { id: "brand-kit",    kanji: "紋", name: "Brand Kit",    ftr: 91 },
];

const DORMANT = [
  { kanji: "筆", name: "Sketch tool", last: "3w" },
  { kanji: "巻", name: "Docs site",   last: "2mo" },
];

function NavItem({ item, active, onClick }) {
  return (
    <button onClick={onClick} className="flex items-center gap-3 w-full"
      style={{
        padding: '7px 10px', borderRadius: 6, textAlign: 'left',
        background: active ? 'var(--paper-3)' : 'transparent',
        color: active ? 'var(--ink)' : 'var(--ink-2)',
        fontSize: 'var(--text-sm)'
      }}>
      <span className="zs-kanji" style={{ fontSize: 13, width: 14, color: active ? 'var(--accent)' : 'var(--ink-3)' }}>
        {item.kanji}
      </span>
      <span style={{ flex: 1 }}>{item.label}</span>
      {item.badge != null && (
        <span className="zs-mono text-xs text-ink-3">{item.badge}</span>
      )}
    </button>
  );
}

function ProjectItem({ p, active, onClick }) {
  return (
    <button onClick={onClick} className="flex items-center gap-3 w-full"
      style={{
        padding: '8px 10px', borderRadius: 6, textAlign: 'left',
        background: active ? 'var(--paper-3)' : 'transparent',
        color: active ? 'var(--ink)' : 'var(--ink-2)',
        fontSize: 'var(--text-sm)'
      }}>
      <span className="zs-kanji" style={{ fontSize: 13, width: 14, color: p.warn ? 'var(--warning)' : 'var(--accent)' }}>
        {p.kanji}
      </span>
      <span style={{ flex: 1 }}>{p.name}</span>
      <span className="zs-mono text-xs" style={{ color: p.warn ? 'var(--warning)' : 'var(--ink-3)' }}>
        {p.ftr}
      </span>
    </button>
  );
}

function Sidebar({ section, setSection }) {
  return (
    <aside className="border-r"
      style={{ width: 240, padding: '20px 12px', display: 'flex',
               flexDirection: 'column', gap: 24, overflow: 'auto',
               background: 'var(--paper)' }}>
      {/* Brand */}
      <div className="flex items-baseline gap-2" style={{ padding: '0 6px' }}>
        <span className="zs-kanji" style={{ fontSize: 20, color: 'var(--accent)' }}>先</span>
        <span style={{ fontFamily: 'var(--font-display)', fontSize: 16 }}>Sensei</span>
      </div>

      {/* Sections */}
      <div>
        <div className="zs-eyebrow" style={{ padding: '0 10px', marginBottom: 8, fontSize: 10 }}>Observatory</div>
        <div className="flex flex-col" style={{ gap: 1 }}>
          {NAV.map(item => (
            <NavItem key={item.id} item={item} active={section === item.id}
                     onClick={() => setSection(item.id)} />
          ))}
        </div>
      </div>

      {/* Active projects */}
      <div>
        <div className="flex items-baseline justify-between" style={{ padding: '0 10px', marginBottom: 8 }}>
          <span className="zs-eyebrow" style={{ fontSize: 10 }}>Active</span>
          <span className="zs-mono text-xs text-ink-4">{ACTIVE.length}</span>
        </div>
        <div className="flex flex-col" style={{ gap: 1 }}>
          {ACTIVE.map(p => <ProjectItem key={p.id} p={p} />)}
        </div>
      </div>

      {/* Dormant projects */}
      <div>
        <div className="zs-eyebrow" style={{ padding: '0 10px', marginBottom: 8, fontSize: 10 }}>Dormant</div>
        <div className="flex flex-col" style={{ gap: 1 }}>
          {DORMANT.map((p, i) => (
            <button key={i} className="flex items-center gap-3 w-full"
              style={{ padding: '7px 10px', borderRadius: 6, textAlign: 'left',
                       color: 'var(--ink-3)', fontSize: 'var(--text-sm)', opacity: 0.82 }}>
              <span className="zs-kanji" style={{ fontSize: 12, width: 14, opacity: 0.6 }}>{p.kanji}</span>
              <span style={{ flex: 1 }}>{p.name}</span>
              <span className="zs-mono text-xs text-ink-4">{p.last}</span>
            </button>
          ))}
        </div>
      </div>

      <div style={{ flex: 1 }}/>

      {/* Daemon status */}
      <div className="border-t" style={{ padding: '12px 10px 0', fontSize: 10, color: 'var(--ink-3)', lineHeight: 1.6 }}>
        <span className="zs-mono">daemon · running</span><br/>
        <span style={{ color: 'var(--ink-4)' }}>last heartbeat 2s ago</span>
      </div>
    </aside>
  );
}

window.Sidebar = Sidebar;
