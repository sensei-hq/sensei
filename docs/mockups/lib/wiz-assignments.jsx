// Sensei — Assignments step (priority per role).
// Comes AFTER Inference. For each reasoning role (inference · consolidation · embedding · voice · fallback)
// the user builds an ordered list of models. First entry is primary; the rest are fallbacks.
//
// Layout: left nav of roles with live counts; right panel for the selected role.
// The right panel is the "RoleBoard" — ordered list of model chips with
// add/remove/reorder, plus a side picker of available models.
//
// Exposes <WizAssignments state upd/>.

const { useState: aUseS, useMemo: aUseM } = React;

const ROLES_A = [
  { id: "inference",     label: "Inference",     kanji: "推",
    hint: "insights · actions · recommendations from sessions + memory" },
  { id: "consolidation", label: "Consolidation", kanji: "洞",
    hint: "merge memories · detect conflicts · propose scope updates" },
  { id: "embedding",     label: "Embedding",     kanji: "印",
    hint: "index sessions, memories, code refs for retrieval" },
  { id: "voice",         label: "Voice",         kanji: "話",
    hint: "sensei speaks & listens — surfaced in the observatory, not here" },
  { id: "fallback",      label: "Fallback",      kanji: "備",
    hint: "takes over when primary is down or rate-limited" }
];

// ═══════════════════════════════════════════════════════════════
// Root
// ═══════════════════════════════════════════════════════════════
function WizAssignments({ state, upd }) {
  const D = window.SENSEI_SETUP.inference;
  const [priority, setPriority] = aUseS(() => ({ ...D.rolePriority }));

  const move = (role, mid, dir) => {
    setPriority(p => {
      const list = [...(p[role] || [])];
      const i = list.indexOf(mid);
      if (i < 0) return p;
      const j = i + dir;
      if (j < 0 || j >= list.length) return p;
      [list[i], list[j]] = [list[j], list[i]];
      return { ...p, [role]: list };
    });
  };
  const remove = (role, mid) =>
    setPriority(p => ({ ...p, [role]: (p[role] || []).filter(x => x !== mid) }));
  const add = (role, mid) =>
    setPriority(p => {
      if ((p[role] || []).includes(mid)) return p;
      return { ...p, [role]: [...(p[role] || []), mid] };
    });

  const props = { D, priority, move, remove, add };

  return (
    <div style={{ maxWidth: 1040, margin: '0 auto' }}>
      <div style={{ marginBottom: 18 }}>
        <WizHeader n="任" title="Assignments"
                   tagline="Decide which models handle each reasoning role. The first model is primary; the rest are fallbacks used if the primary is down or rate-limited."/>
      </div>

      <SplitVariant {...props}/>

      <div style={{ paddingTop: 18, marginTop: 18, borderTop: 'var(--hairline)',
                     display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ fontSize: 12, color: 'var(--sumi-3)', lineHeight: 1.6 }}>
          Voice is optional — if left empty, the observatory voice panel stays hidden.
        </div>
        <button style={{ fontSize: 12, color: 'var(--sumi-2)',
                         padding: '7px 14px', border: 'var(--hairline)',
                         borderRadius: 5, background: 'transparent', cursor: 'pointer' }}>
          Use defaults
        </button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// Split layout — left nav + detail
// ═══════════════════════════════════════════════════════════════
function SplitVariant({ D, priority, move, remove, add }) {
  const [active, setActive] = aUseS(ROLES_A[0].id);
  const role = ROLES_A.find(r => r.id === active);

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '260px 1fr', gap: 18 }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {ROLES_A.map(r => {
          const on = r.id === active;
          const count = (priority[r.id] || []).length;
          const primary = (priority[r.id] || [])[0];
          const primaryName = primary && findModel(D, primary)?.model.name;
          return (
            <button key={r.id} onClick={() => setActive(r.id)}
                    style={{ display: 'grid', gridTemplateColumns: '28px 1fr auto',
                             gap: 10, alignItems: 'center',
                             padding: '12px 12px', borderRadius: 5,
                             border: on ? 'none' : 'var(--hairline)',
                             background: on ? 'var(--sumi)' : 'var(--paper)',
                             color: on ? 'var(--paper)' : 'var(--sumi)',
                             cursor: 'pointer', textAlign: 'left' }}>
              <span className="kanji" style={{ fontSize: 18,
                                                 color: on ? 'var(--paper)' : 'var(--shu)' }}>
                {r.kanji}
              </span>
              <div>
                <div style={{ fontSize: 13 }}>{r.label}</div>
                <div style={{ fontSize: 10.5, opacity: 0.65, marginTop: 2,
                                overflow: 'hidden', textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap' }}>
                  {primaryName || <span style={{ fontStyle: 'italic' }}>— none —</span>}
                </div>
              </div>
              <span style={{ fontSize: 10, fontFeatureSettings: '"tnum"',
                              padding: '2px 7px', borderRadius: 3,
                              background: on ? 'rgba(255,255,255,.18)' : 'var(--paper-2)',
                              color: on ? 'var(--paper)' : 'var(--sumi-3)' }}>
                {count}
              </span>
            </button>
          );
        })}
      </div>

      <RoleBoard role={role} D={D} priority={priority}
                 onMove={move} onRemove={remove} onAdd={add}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// RoleBoard — shared detail panel
// Ordered list on the left, available-models picker on the right
// ═══════════════════════════════════════════════════════════════
function RoleBoard({ role, D, priority, onMove, onRemove, onAdd }) {
  const assigned = priority[role.id] || [];
  const all = aUseM(() => {
    return D.providers.flatMap(p => p.models.map(m => ({ ...m, providerId: p.id,
                                                           providerName: p.name,
                                                           providerKanji: p.kanji })));
  }, []);
  const available = all.filter(m => !assigned.includes(m.id));

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 4 }}>
        <span className="kanji" style={{ fontSize: 24, color: 'var(--shu)' }}>{role.kanji}</span>
        <div className="display" style={{ fontSize: 18 }}>{role.label}</div>
      </div>
      <div style={{ fontSize: 12.5, color: 'var(--sumi-3)', marginBottom: 18 }}>
        {role.hint}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 280px', gap: 14 }}>
        {/* Ordered priority list */}
        <div style={{ background: 'var(--paper)', border: 'var(--hairline)',
                       borderRadius: 6, padding: 14, minHeight: 220 }}>
          <SectionLabel>priority</SectionLabel>
          {assigned.length === 0 ? (
            <div style={{ padding: '20px 0', textAlign: 'center',
                           color: 'var(--sumi-4)', fontSize: 12, fontStyle: 'italic' }}>
              No models assigned — add one from the right →
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {assigned.map((mid, i) => {
                const found = findModel(D, mid);
                if (!found) return null;
                return (
                  <PriorityRow key={mid}
                               model={found.model}
                               provider={found.provider}
                               position={i}
                               canUp={i > 0}
                               canDown={i < assigned.length - 1}
                               onUp={() => onMove(role.id, mid, -1)}
                               onDown={() => onMove(role.id, mid, 1)}
                               onRemove={() => onRemove(role.id, mid)}/>
                );
              })}
            </div>
          )}
        </div>

        {/* Available picker */}
        <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 6, padding: 14, minHeight: 220 }}>
          <SectionLabel>add model</SectionLabel>
          <AvailableList models={available} onAdd={(mid) => onAdd(role.id, mid)}/>
        </div>
      </div>
    </div>
  );
}

function PriorityRow({ model, provider, position, canUp, canDown, onUp, onDown, onRemove }) {
  const primary = position === 0;
  return (
    <div style={{ display: 'grid',
                   gridTemplateColumns: '22px 28px 1fr auto auto',
                   gap: 8, alignItems: 'center',
                   padding: '8px 10px',
                   background: primary ? 'var(--sumi)' : 'var(--paper-2)',
                   color: primary ? 'var(--paper)' : 'var(--sumi)',
                   border: primary ? 'none' : 'var(--hairline)',
                   borderRadius: 4 }}>
      <span style={{ fontSize: 10, opacity: 0.6,
                      fontFeatureSettings: '"tnum"',
                      textAlign: 'center' }}>{position + 1}</span>
      <span className="kanji" style={{ fontSize: 13,
                                         color: primary ? 'var(--paper)' : 'var(--shu)' }}>
        {provider.kanji}
      </span>
      <div>
        <div style={{ fontSize: 12.5 }}>{model.name}</div>
        <div style={{ fontSize: 10, opacity: 0.65 }}>
          {provider.name}{primary && <span style={{ marginLeft: 8,
                                                      fontSize: 9,
                                                      letterSpacing: '0.1em',
                                                      textTransform: 'uppercase',
                                                      opacity: 0.8 }}>primary</span>}
        </div>
      </div>
      <div style={{ display: 'flex', gap: 2 }}>
        <IconBtn onClick={onUp} disabled={!canUp} primary={primary}>▲</IconBtn>
        <IconBtn onClick={onDown} disabled={!canDown} primary={primary}>▼</IconBtn>
      </div>
      <IconBtn onClick={onRemove} primary={primary}>×</IconBtn>
    </div>
  );
}

function IconBtn({ onClick, disabled, primary, children }) {
  return (
    <button onClick={onClick} disabled={disabled}
            style={{ width: 22, height: 22, border: 'none',
                     background: 'transparent',
                     color: disabled
                              ? (primary ? 'rgba(255,255,255,.22)' : 'var(--sumi-4)')
                              : (primary ? 'var(--paper)' : 'var(--sumi-2)'),
                     fontSize: 11,
                     cursor: disabled ? 'default' : 'pointer',
                     borderRadius: 3,
                     display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>
      {children}
    </button>
  );
}

function AvailableList({ models, onAdd }) {
  if (models.length === 0) {
    return (
      <div style={{ padding: '20px 0', textAlign: 'center',
                     color: 'var(--sumi-4)', fontSize: 12, fontStyle: 'italic' }}>
        All models assigned
      </div>
    );
  }
  // Group by provider
  const byProvider = models.reduce((a, m) => {
    (a[m.providerId] = a[m.providerId] || { name: m.providerName, kanji: m.providerKanji,
                                              models: [] }).models.push(m);
    return a;
  }, {});
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {Object.entries(byProvider).map(([pid, g]) => (
        <div key={pid}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 5 }}>
            <span className="kanji" style={{ fontSize: 12, color: 'var(--shu)' }}>{g.kanji}</span>
            <span style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                            letterSpacing: '0.08em', textTransform: 'uppercase' }}>
              {g.name}
            </span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            {g.models.map(m => (
              <button key={m.id} onClick={() => onAdd(m.id)}
                      style={{ display: 'grid',
                               gridTemplateColumns: '1fr auto', gap: 6,
                               alignItems: 'center',
                               padding: '7px 10px',
                               background: 'transparent',
                               border: 'none', borderRadius: 4,
                               cursor: 'pointer', textAlign: 'left',
                               color: 'var(--sumi)' }}
                      onMouseEnter={e => e.currentTarget.style.background = 'var(--paper)'}
                      onMouseLeave={e => e.currentTarget.style.background = 'transparent'}>
                <span style={{ fontSize: 12 }}>{m.name}</span>
                <span style={{ fontSize: 14, color: 'var(--sumi-3)' }}>+</span>
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function findModel(D, modelId) {
  for (const p of D.providers) {
    const m = p.models.find(x => x.id === modelId);
    if (m) return { provider: p, model: m };
  }
  return null;
}

Object.assign(window, { WizAssignments });
