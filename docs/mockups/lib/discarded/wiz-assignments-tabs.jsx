// DISCARDED — Tabs variant of the Assignments step + its variant switcher.
// Kept for reference. Not loaded in the final design.
//
// Original shape: role tabs across the top; clicking a tab showed that
// role's ordered model list below. Superseded by the Split layout (left
// nav of roles with live counts, right-hand detail panel) because:
//   - Split surfaces every role's count at a glance without switching tabs.
//   - The ordered-list detail is vertical; vertical detail pairs better
//     with a vertical role nav than with horizontal tabs.
//
// Both variants share the RoleBoard component (ordered chips + picker).

function AsgToggle({ variant, onChange }) {
  return (
    <div style={{ flexShrink: 0 }}>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-4)', textAlign: 'right'
}} className="mb-1" >variant</div>
      <div style={{
 display: 'flex', background: 'var(--paper-2)',
                     borderRadius: 5, border: 'var(--hairline)'
}} className="p-1 gap-0" >
        {[{ id: "A", label: "Tabs" }, { id: "B", label: "Split" }].map(v => (
          <button key={v.id} onClick={() => onChange(v.id)}
                  style={{
 fontSize: 11, borderRadius: 3,
                           background: variant === v.id ? 'var(--paper)' : 'transparent',
                           color: variant === v.id ? 'var(--ink)' : 'var(--ink-3)',
                           border: 'none', cursor: 'pointer', letterSpacing: '0.04em'
}} className="py-1 px-3" >
            {v.id} · {v.label}
          </button>
        ))}
      </div>
    </div>
  );
}

// Tabs variant — role tabs across the top, detail panel below.
function TabsVariant({ D, priority, move, remove, add }) {
  const [active, setActive] = aUseS(ROLES_A[0].id);
  const role = ROLES_A.find(r => r.id === active);

  return (
    <>
      <div style={{
 display: 'flex', borderBottom: 'var(--hairline)'
}} className="mb-4 gap-0" >
        {ROLES_A.map(r => {
          const on = r.id === active;
          const count = (priority[r.id] || []).length;
          return (
            <button key={r.id} onClick={() => setActive(r.id)}
                    style={{
 position: 'relative',
                             display: 'flex', alignItems: 'center',
                             background: 'transparent', border: 'none',
                             borderBottom: on ? '2px solid var(--ink)' : '2px solid transparent',
                             marginBottom: -1,
                             color: on ? 'var(--ink)' : 'var(--ink-3)',
                             cursor: 'pointer'
}} className="gap-2 py-3 px-4" >
              <span className="kanji" style={{ fontSize: 15,
                                                 color: on ? 'var(--accent)' : 'var(--ink-3)' }}>
                {r.kanji}
              </span>
              <span className="display" style={{ fontSize: 13 }}>{r.label}</span>
              <span style={{
 fontSize: 11, color: 'var(--ink-4)',
                              fontFeatureSettings: '"tnum"', borderRadius: 3,
                              background: on ? 'var(--paper-2)' : 'transparent'
}} className="py-1 px-1" >
                {count}
              </span>
            </button>
          );
        })}
      </div>

      <RoleBoard role={role} D={D} priority={priority}
                 onMove={move} onRemove={remove} onAdd={add}/>
    </>
  );
}
