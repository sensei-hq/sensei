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
  // Every local router (Embedded Ollama, Ollama, …) has pullable models.
  const localModels = D.providers.filter(p => p.kind === "local").flatMap(p => p.models);
  const [progress, setProgress] = iUseS(() =>
    localModels.reduce((a, m) => (a[m.id] = m.pulled ? 100 : 0, a), {})
  );
  const [pullQueue, setPullQueue] = iUseS(() =>
    localModels.reduce((a, m) => (a[m.id] = !!m.recommended || !!m.pulled, a), {})
  );
  const [showAdd, setShowAdd] = iUseS(false);

  // Tick pull progress across all local routers
  iUseE(() => {
    const lm = D.providers.filter(p => p.kind === "local").flatMap(p => p.models);
    const t = setInterval(() => {
      setProgress(p => {
        const next = { ...p };
        lm.forEach(m => {
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
    <div style={{ maxWidth: 980 }} className="mx-auto" >
      <div className="mb-4" >
        <WizHeader n="路" title="Routers"
                   tagline="Routers give sensei models for reasoning — inferring insights, consolidating memory, and making recommendations. Add a router, pull local models, leave assignment for the next step."/>
      </div>

      <SystemStrip sys={s.sys}/>

      <InferenceSplit {...s}/>

      <div className="mt-4 pt-4 border-t flex justify-between items-center" >
        <div className="text-ink-3" style={{ fontSize: 13, lineHeight: 1.6 }}>
          Role assignments come next — decide which models handle inference, consolidation, embedding, voice, and image.
        </div>
        <button style={{
 fontSize: 13,
 borderRadius: 5 }} className="py-2 px-3 text-ink-2 border border-paper-edge bg-transparent cursor-pointer" >
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
    <div style={{
 borderRadius: 6 }} className="mb-6 py-3 px-4 gap-6 bg-paper-2 border border-paper-edge flex items-center flex-wrap" >
      <div className="uppercase text-ink-4" style={{ fontSize: 11, letterSpacing: '0.14em' }}>this machine</div>
      {[sys.chip, sys.ram, sys.cores.split('·')[1]?.trim() || sys.cores, sys.os].map((v, i, arr) => (
        <React.Fragment key={i}>
          <div className="text-ink-2" style={{ fontSize: 13 }}>{v}</div>
          {i < arr.length - 1 && <span className="text-ink-4" >·</span>}
        </React.Fragment>
      ))}
    </div>
  );
}

function KeyInput({ envVar, value, onChange, onSave }) {
  return (
    <div className="mb-3" >
      <div style={{ fontSize: 11 }} className="mb-1 text-ink-3" >
        Paste your API key (or export <span style={{ fontFamily: 'var(--font-mono)' }}>{envVar}</span> in your shell):
      </div>
      <div className="gap-2 flex" >
        <input type="password" value={value}
 onChange={e => onChange(e.target.value)}
 placeholder={envVar.toLowerCase()}
 style={{ fontSize: 13, fontFamily: 'var(--font-mono)', borderRadius: 4 }} className="py-2 px-2 flex-1 border border-paper-edge bg-paper text-ink" />
        <button onClick={onSave} disabled={!value}
 style={{
 fontSize: 13, borderRadius: 4,
 background: value ? 'var(--ink)' : 'var(--edge)',
 color: value ? 'var(--paper)' : 'var(--ink-3)',
 cursor: value ? 'pointer' : 'default'
 }} className="py-2 px-3 border-0" >
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
          <div className="gap-1 mb-3 flex flex-col" >
            {recs.map(m => <OllamaRow key={m.id} m={m} progress={progress}
                                       pullQueue={pullQueue} setPullQueue={setPullQueue}/>)}
          </div>
        </>
      )}
      {rest.length > 0 && (
        <>
          <SectionLabel>also available</SectionLabel>
          <div className="gap-1 flex flex-col" >
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
    <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 uppercase text-ink-4" >{children}</div>
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
    <span className="inline-flex items-center justify-center bg-success-soft text-success" title="pulled"
 style={{
 width: 22, height: 22, borderRadius: 4, fontSize: 13 }}>✓</span>
  ) : active ? (
    <span style={{
 fontSize: 11,
 fontFeatureSettings: '"tnum"',
 letterSpacing: '0.04em'
 }} className="gap-1 inline-flex items-center text-ink-3" >
      <Spinner/>
      {Math.round(prog)}%
    </span>
  ) : (
    <button onClick={() => setPullQueue(q => ({ ...q, [m.id]: !q[m.id] }))}
 style={{
 fontSize: 11, borderRadius: 3,
 letterSpacing: '0.04em' }}
 onMouseEnter={e => e.currentTarget.style.color = 'var(--ink)'}
 onMouseLeave={e => e.currentTarget.style.color = 'var(--ink-2)'} className="py-1 px-3 border-0 bg-transparent text-ink-2 cursor-pointer" >
      pull
    </button>
  );

  return (
    <div style={{ gridTemplateColumns: '1fr auto auto',
 borderRadius: 4 }} className="gap-3 py-2 px-3 grid items-center bg-paper-2 relative" >
      {m.recommended && <RecommendedBadge/>}
      <div style={{ paddingLeft: m.recommended ? 10 : 0 }}>
        <div className="gap-2 flex items-baseline" >
          <span style={{ fontSize: 13 }}>{m.name}</span>
        </div>
        {m.note && <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >{m.note}</div>}
        {active && (
          <div className="mt-1" >
            <div className="overflow-hidden" style={{ height: 2, background: 'var(--edge)',
 borderRadius: 1, maxWidth: 240 }}>
              <div className="h-full bg-success" style={{ width: `${prog}%`,
 transition: 'width .22s linear' }}/>
            </div>
          </div>
        )}
      </div>
      <div className="text-ink-4 whitespace-nowrap" style={{ fontSize: 11,
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
    <span className="inline-block rounded-full" style={{ width: 10, height: 10,
 border: '1.5px solid var(--edge)',
 borderTopColor: 'var(--ink-2)',
 animation: 'senseiSpin 0.8s linear infinite' }}/>
  );
}

function RecommendedBadge() {
  // subtle corner badge — a small notch, not a pill
  return (
    <span className="absolute" title="Recommended for this machine"
 style={{ top: 0, left: 0,
 width: 0, height: 0,
 borderTop: '18px solid var(--accent)',
 borderRight: '18px solid transparent' }}/>
  );
}

function CloudModelTable({ models, isConfigured }) {
  return (
    <>
      <SectionLabel>models</SectionLabel>
      <div className="gap-1 flex flex-col" >
        {models.map(m => (
          <div key={m.id} style={{ gridTemplateColumns: '1fr auto', borderRadius: 4,
 opacity: isConfigured ? 1 : 0.5
 }} className="gap-3 py-2 px-3 grid items-center bg-paper-2" >
            <div>
              <div className="gap-2 flex items-baseline" >
                <span style={{ fontSize: 13 }}>{m.name}</span>
                {m.context && <span className="text-ink-4" style={{ fontSize: 11 }}>{m.context}</span>}
              </div>
              {m.cost && (
                <div style={{
 fontSize: 11,
 fontFamily: 'var(--font-mono)'
 }} className="mt-1 text-ink-4" >{m.cost}</div>
              )}
            </div>
            <span className="uppercase" style={{ fontSize: 11, color: isConfigured ? 'var(--success)' : 'var(--ink-4)',
 letterSpacing: '0.08em' }}>
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
      <div style={{ gridTemplateColumns: '280px 1fr',
 minHeight: 380
 }} className="gap-3 grid" >
        {/* Left list */}
        <div>
          <div className="mb-2 flex items-center justify-between" >
            <SectionLabel>routers</SectionLabel>
            <button onClick={() => setShowAdd(true)}
 style={{
 fontSize: 11, borderRadius: 3 }} className="py-1 px-2 text-ink-2 border border-paper-edge bg-paper cursor-pointer" >+ add</button>
          </div>
          <div className="gap-1 flex flex-col" >
            {D.providers.map(p => {
              const active = p.id === focusId;
              const cfg = configured[p.id];
              const count = p.kind === "local"
                ? p.models.filter(m => m.pulled || pullQueue[m.id]).length
                : (cfg ? p.models.length : 0);
              return (
                <button key={p.id} onClick={() => setFocusId(p.id)}
 style={{
 gridTemplateColumns: '24px 1fr auto', borderRadius: 5,
 border: active ? 'none' : 'var(--hairline)',
 background: active ? 'var(--ink)' : 'var(--paper)',
 color: active ? 'var(--paper)' : 'var(--ink)' }} className="gap-2 py-2 px-3 grid items-center cursor-pointer text-left" >
                  <span className="kanji" style={{ fontSize: 15,
                                                     color: active ? 'var(--paper)' : 'var(--accent)' }}>
                    {p.kanji}
                  </span>
                  <div>
                    <div style={{ fontSize: 13 }}>{p.name}</div>
                    <div style={{ fontSize: 11, opacity: 0.6 }} className="mt-1" >
                      {p.kind === "local" ? "local" : "cloud"}
                    </div>
                  </div>
                  <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.08em',
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
        <div style={{
 borderRadius: 6
 }} className="py-4 px-4 bg-paper border border-paper-edge" >
          <div className="gap-2 mb-1 flex items-baseline" >
            <span className="kanji text-accent" style={{ fontSize: 22 }}>{focus.kanji}</span>
            <div className="display" style={{ fontSize: 17 }}>{focus.name}</div>
            <span className="uppercase text-ink-4" style={{ fontSize: 11, letterSpacing: '0.1em' }}>
              {focus.kind === "local" ? "local" : "cloud"}
            </span>
          </div>
          <div style={{ fontSize: 13 }} className="mb-3 text-ink-3" >
            {focus.note}
          </div>

          {!configured[focus.id] && focus.envVar && (
            <SplitKeyInput envVar={focus.envVar}
                           value={keys[focus.id] || ""}
                           onChange={(v) => setKeys(k => ({ ...k, [focus.id]: v }))}
                           onSave={() => setConfigured(c => ({ ...c, [focus.id]: true }))}/>
          )}

          {focus.kind === "local" ? (
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
    <div style={{
 borderRadius: 5
 }} className="mb-3 p-3 bg-paper-2 border border-paper-edge" >
      <KeyInput {...p}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// Add-provider modal
// ═══════════════════════════════════════════════════════════════
function AddProviderModal({ D, onAdd, onClose }) {
  return (
    <div className="absolute grid" onClick={onClose}
 style={{ inset: 0, background: 'var(--scrim)', placeItems: 'center', zIndex: 20 }}>
      <div onClick={e => e.stopPropagation()}
 style={{
 borderRadius: 8, width: 420 }} className="p-6 bg-paper border border-paper-edge shadow-lg" >
        <div className="display mb-1" style={{ fontSize: 15 }}>Add router</div>
        <div style={{ fontSize: 13 }} className="mb-4 text-ink-3" >
          Pick a router; paste a key on the next step.
        </div>
        <div className="gap-1 flex flex-col" >
          {D.addable.map(p => (
            <button key={p.id} onClick={() => onAdd(p.id)}
 style={{ borderRadius: 4 }} className="gap-3 py-2 px-3 flex items-center border border-paper-edge bg-paper cursor-pointer text-left" >
              <span className="kanji text-accent" style={{ fontSize: 17 }}>{p.kanji}</span>
              <span className="flex-1" style={{ fontSize: 13 }}>{p.name}</span>
              <span className="text-ink-4 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.08em' }}>{p.kind}</span>
            </button>
          ))}
        </div>
        <div className="mt-3 text-right" >
          <button onClick={onClose}
 style={{
 fontSize: 13 }} className="py-1 px-2 text-ink-3 border-0 bg-transparent cursor-pointer" >Cancel</button>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { WizInference });
