// Sensei — Inference step (Providers & Models).
// Scope: list configured providers, let user add/configure more, and manage which
// models are installed or available. Role ASSIGNMENTS live in the next step.
//
// Two variants of this step (toggle in the top-right of the step):
//   A · Ladder        — providers as expandable cards, models listed inside.
//   B · Split columns — provider list on the left, focused provider detail on the right.
//
// Shared concepts:
//   · Providers include Ollama (local) alongside cloud providers (Anthropic, OpenAI, Google…).
//   · Detected env keys auto-configure their provider.
//   · User can add a provider and paste a key.
//   · Ollama models "pull"; cloud models "enable".
//   · Recommended models carry a subtle badge (not a pill).
//
// Global entry: <WizInference state upd/>

const { useState: iUseS, useEffect: iUseE, useMemo: iUseM } = React;

function useInferenceState() {
  const D = window.SENSEI_SETUP.inference;
  const sys = D.system;

  const [configured, setConfigured] = iUseS(() =>
    D.providers.reduce((a, p) => (a[p.id] = !!p.configured, a), {})
  );
  const [keys, setKeys] = iUseS(() =>
    D.providers.reduce((a, p) => (a[p.id] = "", a), {})
  );
  const [progress, setProgress] = iUseS(() =>
    D.providers.find(p => p.id === "ollama").models
      .reduce((a, m) => (a[m.id] = m.pulled ? 100 : 0, a), {})
  );
  const [pullQueue, setPullQueue] = iUseS(() =>
    D.providers.find(p => p.id === "ollama").models
      .reduce((a, m) => (a[m.id] = !!m.recommended || !!m.pulled, a), {})
  );
  const [showAdd, setShowAdd] = iUseS(false);

  // Tick pull progress
  iUseE(() => {
    const ollama = D.providers.find(p => p.id === "ollama");
    const t = setInterval(() => {
      setProgress(p => {
        const next = { ...p };
        ollama.models.forEach(m => {
          if (m.pulled) { next[m.id] = 100; return; }
          if (pullQueue[m.id] && next[m.id] < 100) {
            const bump = Math.max(0.8, 6 - m.sizeGB * 0.12);
            next[m.id] = Math.min(100, (next[m.id] || 0) + bump);
          }
        });
        return next;
      });
    }, 220);
    return () => clearInterval(t);
  }, [pullQueue]);

  return {
    D, sys,
    configured, setConfigured, keys, setKeys,
    progress, pullQueue, setPullQueue,
    showAdd, setShowAdd
  };
}

// ═══════════════════════════════════════════════════════════════
// Root dispatcher
// ═══════════════════════════════════════════════════════════════
function WizInference({ state, upd }) {
  const s = useInferenceState();

  return (
    <div style={{ maxWidth: 980, margin: '0 auto' }}>
      <div style={{ marginBottom: 16 }}>
        <WizHeader n="想" title="Inference"
                   tagline="Providers give sensei models for reasoning — inferring insights, consolidating memory, and making recommendations. Add providers, pull local models, leave assignment for the next step."/>
      </div>

      <SystemStrip sys={s.sys}/>

      <InferenceSplit {...s}/>

      <div style={{ paddingTop: 16, marginTop: 16, borderTop: 'var(--hairline)',
                     display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.6 }}>
          Role assignments come next — decide which models handle inference, consolidation, embedding, voice, and fallback.
        </div>
        <button style={{ fontSize: 13, color: 'var(--ink-2)',
                         padding: '8px 12px', border: 'var(--hairline)',
                         borderRadius: 5, background: 'transparent', cursor: 'pointer' }}>
          Defer · configure later
        </button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// Shared pieces
// ═══════════════════════════════════════════════════════════════

function SystemStrip({ sys }) {
  return (
    <div style={{ marginBottom: 24, padding: '12px 16px',
                   background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 6,
                   display: 'flex', alignItems: 'center', gap: 24, flexWrap: 'wrap' }}>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-4)' }}>this machine</div>
      {[sys.chip, sys.ram, sys.cores.split('·')[1]?.trim() || sys.cores, sys.os].map((v, i, arr) => (
        <React.Fragment key={i}>
          <div style={{ fontSize: 13, color: 'var(--ink-2)' }}>{v}</div>
          {i < arr.length - 1 && <span style={{ color: 'var(--ink-4)' }}>·</span>}
        </React.Fragment>
      ))}
    </div>
  );
}

function KeyInput({ envVar, value, onChange, onSave }) {
  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', marginBottom: 4 }}>
        Paste your API key (or export <span style={{ fontFamily: 'var(--font-mono)' }}>{envVar}</span> in your shell):
      </div>
      <div style={{ display: 'flex', gap: 8 }}>
        <input type="password" value={value}
               onChange={e => onChange(e.target.value)}
               placeholder={envVar.toLowerCase()}
               style={{ flex: 1, fontSize: 13, fontFamily: 'var(--font-mono)',
                        padding: '8px 8px', borderRadius: 4,
                        border: 'var(--hairline)', background: 'var(--paper)',
                        color: 'var(--ink)' }}/>
        <button onClick={onSave} disabled={!value}
                style={{ fontSize: 13, padding: '8px 12px', borderRadius: 4, border: 'none',
                         background: value ? 'var(--ink)' : 'var(--edge)',
                         color: value ? 'var(--paper)' : 'var(--ink-3)',
                         cursor: value ? 'pointer' : 'default' }}>
          Configure
        </button>
      </div>
    </div>
  );
}

// Ollama models: recommended grouped on top with a badge on the row
function OllamaModelTable({ models, progress, pullQueue, setPullQueue }) {
  const recs = models.filter(m => m.recommended);
  const rest = models.filter(m => !m.recommended);

  return (
    <>
      {recs.length > 0 && (
        <>
          <SectionLabel>recommended for this machine</SectionLabel>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 12 }}>
            {recs.map(m => <OllamaRow key={m.id} m={m} progress={progress}
                                       pullQueue={pullQueue} setPullQueue={setPullQueue}/>)}
          </div>
        </>
      )}
      {rest.length > 0 && (
        <>
          <SectionLabel>also available</SectionLabel>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {rest.map(m => <OllamaRow key={m.id} m={m} progress={progress}
                                        pullQueue={pullQueue} setPullQueue={setPullQueue}/>)}
          </div>
        </>
      )}
    </>
  );
}

function SectionLabel({ children }) {
  return (
    <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                   color: 'var(--ink-4)', marginBottom: 8 }}>{children}</div>
  );
}

function OllamaRow({ m, progress, pullQueue, setPullQueue }) {
  const on = !!pullQueue[m.id];
  const prog = progress[m.id] || 0;
  const done = m.pulled || prog >= 100;
  const active = on && !done;

  // Single control that morphs through three states:
  //   idle     → "pull" (ghost button)
  //   pulling  → spinner + % (ghost, disabled)
  //   pulled   → checkbox-style ✓ (subtle matcha)
  const ctrl = done ? (
    <span title="pulled"
          style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                    width: 22, height: 22, borderRadius: 4,
                    background: 'rgba(122,158,98,.12)',
                    color: 'var(--success)', fontSize: 13 }}>✓</span>
  ) : active ? (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4,
                    fontSize: 11, color: 'var(--ink-3)',
                    fontFeatureSettings: '"tnum"',
                    letterSpacing: '0.04em' }}>
      <Spinner/>
      {Math.round(prog)}%
    </span>
  ) : (
    <button onClick={() => setPullQueue(q => ({ ...q, [m.id]: !q[m.id] }))}
            style={{ fontSize: 11, padding: '4px 12px', borderRadius: 3,
                     border: 'none', background: 'transparent',
                     color: 'var(--ink-2)',
                     letterSpacing: '0.04em',
                     cursor: 'pointer' }}
            onMouseEnter={e => e.currentTarget.style.color = 'var(--ink)'}
            onMouseLeave={e => e.currentTarget.style.color = 'var(--ink-2)'}>
      pull
    </button>
  );

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr auto auto', gap: 12,
                   alignItems: 'center', padding: '8px 12px',
                   background: 'var(--paper-2)',
                   borderRadius: 4,
                   position: 'relative' }}>
      {m.recommended && <RecommendedBadge/>}
      <div style={{ paddingLeft: m.recommended ? 10 : 0 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
          <span style={{ fontSize: 13 }}>{m.name}</span>
        </div>
        {m.note && <div style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>{m.note}</div>}
        {active && (
          <div style={{ marginTop: 4 }}>
            <div style={{ height: 2, background: 'var(--edge)',
                           borderRadius: 1, overflow: 'hidden', maxWidth: 240 }}>
              <div style={{ width: `${prog}%`, height: '100%', background: 'var(--success)',
                             transition: 'width .22s linear' }}/>
            </div>
          </div>
        )}
      </div>
      <div style={{ fontSize: 11, color: 'var(--ink-4)',
                     whiteSpace: 'nowrap',
                     letterSpacing: '0.06em',
                     fontFeatureSettings: '"tnum"' }}>
        {m.sizeGB.toFixed(1)} GB
      </div>
      {ctrl}
    </div>
  );
}

function Spinner() {
  return (
    <span style={{ display: 'inline-block', width: 10, height: 10,
                    border: '1.5px solid var(--edge)',
                    borderTopColor: 'var(--ink-2)',
                    borderRadius: '50%',
                    animation: 'senseiSpin 0.8s linear infinite' }}/>
  );
}

function RecommendedBadge() {
  // subtle corner badge — a small notch, not a pill
  return (
    <span title="Recommended for this machine"
          style={{ position: 'absolute', top: 0, left: 0,
                    width: 0, height: 0,
                    borderTop: '18px solid var(--accent)',
                    borderRight: '18px solid transparent' }}/>
  );
}

function CloudModelTable({ models, isConfigured }) {
  return (
    <>
      <SectionLabel>models</SectionLabel>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {models.map(m => (
          <div key={m.id} style={{
                 display: 'grid', gridTemplateColumns: '1fr auto', gap: 12,
                 alignItems: 'center', padding: '8px 12px',
                 background: 'var(--paper-2)', borderRadius: 4,
                 opacity: isConfigured ? 1 : 0.5 }}>
            <div>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
                <span style={{ fontSize: 13 }}>{m.name}</span>
                {m.context && <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>{m.context}</span>}
              </div>
              {m.cost && (
                <div style={{ fontSize: 11, color: 'var(--ink-4)',
                               fontFamily: 'var(--font-mono)', marginTop: 4 }}>{m.cost}</div>
              )}
            </div>
            <span style={{ fontSize: 11, color: isConfigured ? 'var(--success)' : 'var(--ink-4)',
                            letterSpacing: '0.08em', textTransform: 'uppercase' }}>
              {isConfigured ? "available" : "needs key"}
            </span>
          </div>
        ))}
      </div>
    </>
  );
}

// ═══════════════════════════════════════════════════════════════
// VARIANT B · Split — list on left, detail on right
// ═══════════════════════════════════════════════════════════════
function InferenceSplit(s) {
  const { D, configured, setConfigured, keys, setKeys,
          progress, pullQueue, setPullQueue, showAdd, setShowAdd } = s;
  const [focusId, setFocusId] = iUseS(D.providers[0].id);
  const focus = D.providers.find(p => p.id === focusId);

  return (
    <>
      <div style={{ display: 'grid', gridTemplateColumns: '280px 1fr', gap: 12,
                     minHeight: 380 }}>
        {/* Left list */}
        <div>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                         marginBottom: 8 }}>
            <SectionLabel>providers</SectionLabel>
            <button onClick={() => setShowAdd(true)}
                    style={{ fontSize: 11, color: 'var(--ink-2)', padding: '4px 8px',
                             border: 'var(--hairline)', borderRadius: 3,
                             background: 'var(--paper)', cursor: 'pointer' }}>+ add</button>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {D.providers.map(p => {
              const active = p.id === focusId;
              const cfg = configured[p.id];
              const count = p.id === "ollama"
                ? p.models.filter(m => m.pulled || pullQueue[m.id]).length
                : (cfg ? p.models.length : 0);
              return (
                <button key={p.id} onClick={() => setFocusId(p.id)}
                        style={{ display: 'grid',
                                 gridTemplateColumns: '24px 1fr auto', gap: 8,
                                 alignItems: 'center',
                                 padding: '8px 12px', borderRadius: 5,
                                 border: active ? 'none' : 'var(--hairline)',
                                 background: active ? 'var(--ink)' : 'var(--paper)',
                                 color: active ? 'var(--paper)' : 'var(--ink)',
                                 cursor: 'pointer', textAlign: 'left' }}>
                  <span className="kanji" style={{ fontSize: 15,
                                                     color: active ? 'var(--paper)' : 'var(--accent)' }}>
                    {p.kanji}
                  </span>
                  <div>
                    <div style={{ fontSize: 13 }}>{p.name}</div>
                    <div style={{ fontSize: 11, opacity: 0.6, marginTop: 4 }}>
                      {p.kind === "local" ? "local" : "cloud"}
                    </div>
                  </div>
                  <span style={{ fontSize: 11, letterSpacing: '0.08em',
                                  textTransform: 'uppercase',
                                  color: cfg
                                    ? (active ? 'var(--paper)' : 'var(--success)')
                                    : (active ? 'var(--paper)' : 'var(--ink-4)') }}>
                    {cfg ? `${count}` : "—"}
                  </span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Right detail */}
        <div style={{ background: 'var(--paper)', border: 'var(--hairline)',
                       borderRadius: 6, padding: '16px 16px' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8,
                         marginBottom: 4 }}>
            <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>{focus.kanji}</span>
            <div className="display" style={{ fontSize: 17 }}>{focus.name}</div>
            <span style={{ fontSize: 11, letterSpacing: '0.1em', textTransform: 'uppercase',
                            color: 'var(--ink-4)' }}>
              {focus.kind === "local" ? "local · ollama" : "cloud"}
            </span>
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-3)', marginBottom: 12 }}>
            {focus.note}
          </div>

          {!configured[focus.id] && focus.envVar && (
            <SplitKeyInput envVar={focus.envVar}
                           value={keys[focus.id] || ""}
                           onChange={(v) => setKeys(k => ({ ...k, [focus.id]: v }))}
                           onSave={() => setConfigured(c => ({ ...c, [focus.id]: true }))}/>
          )}

          {focus.id === "ollama" ? (
            <OllamaModelTable models={focus.models}
                              progress={progress}
                              pullQueue={pullQueue}
                              setPullQueue={setPullQueue}/>
          ) : (
            <CloudModelTable models={focus.models}
                             isConfigured={configured[focus.id]}/>
          )}
        </div>
      </div>

      {showAdd && <AddProviderModal D={D}
                                    onAdd={() => setShowAdd(false)}
                                    onClose={() => setShowAdd(false)}/>}
    </>
  );
}

function SplitKeyInput(p) {
  return (
    <div style={{ marginBottom: 12, padding: 12,
                   background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 5 }}>
      <KeyInput {...p}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// Add-provider modal
// ═══════════════════════════════════════════════════════════════
function AddProviderModal({ D, onAdd, onClose }) {
  return (
    <div onClick={onClose}
         style={{ position: 'absolute', inset: 0, background: 'rgba(30,27,24,.35)',
                   display: 'grid', placeItems: 'center', zIndex: 20 }}>
      <div onClick={e => e.stopPropagation()}
           style={{ background: 'var(--paper)', border: 'var(--hairline)',
                     borderRadius: 8, padding: 24, width: 420,
                     boxShadow: '0 16px 40px rgba(0,0,0,.18)' }}>
        <div className="display" style={{ fontSize: 15, marginBottom: 4 }}>Add provider</div>
        <div style={{ fontSize: 13, color: 'var(--ink-3)', marginBottom: 16 }}>
          Pick a provider; paste a key on the next step.
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {D.addable.map(p => (
            <button key={p.id} onClick={() => onAdd(p.id)}
                    style={{ display: 'flex', alignItems: 'center', gap: 12,
                             padding: '8px 12px', borderRadius: 4,
                             border: 'var(--hairline)', background: 'var(--paper)',
                             cursor: 'pointer', textAlign: 'left' }}>
              <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)' }}>{p.kanji}</span>
              <span style={{ flex: 1, fontSize: 13 }}>{p.name}</span>
              <span style={{ fontSize: 11, color: 'var(--ink-4)',
                              letterSpacing: '0.08em', textTransform: 'uppercase' }}>{p.kind}</span>
            </button>
          ))}
        </div>
        <div style={{ marginTop: 12, textAlign: 'right' }}>
          <button onClick={onClose}
                  style={{ fontSize: 13, color: 'var(--ink-3)',
                           border: 'none', background: 'transparent', padding: '4px 8px',
                           cursor: 'pointer' }}>Cancel</button>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { WizInference });
