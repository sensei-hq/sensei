// DISCARDED — Ladder variant of the Inference step.
// Kept for reference. Not loaded in the final design.
//
// Original shape: providers rendered as stacked expandable cards; clicking
// one expanded it to reveal its models inline. Superseded by the Split
// layout (provider list on the left, detail on the right), which handles
// 6+ providers more gracefully and keeps the models for the focused
// provider always in view.
//
// Also includes VariantToggle, the A/B switcher that used to sit in the
// step header — removed once Ladder was cut.

function VariantToggle({ variant, onChange, options }) {
  return (
    <div style={{ flexShrink: 0 }}>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-4)', marginBottom: 4, textAlign: 'right' }}>variant</div>
      <div style={{ display: 'flex', gap: 0, background: 'var(--paper-2)', padding: 4,
                     borderRadius: 5, border: 'var(--hairline)' }}>
        {options.map(v => (
          <button key={v.id} onClick={() => onChange(v.id)}
                  style={{ padding: '4px 12px', fontSize: 11, borderRadius: 3,
                           background: variant === v.id ? 'var(--paper)' : 'transparent',
                           color: variant === v.id ? 'var(--ink)' : 'var(--ink-3)',
                           border: 'none', cursor: 'pointer', letterSpacing: '0.04em' }}>
            {v.id} · {v.label}
          </button>
        ))}
      </div>
    </div>
  );
}

// Ladder layout — providers as stacked expandable cards.
function InferenceLadder(s) {
  const { D, configured, setConfigured, keys, setKeys,
          progress, pullQueue, setPullQueue, showAdd, setShowAdd } = s;

  return (
    <>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     marginBottom: 8 }}>
        <h3 className="display" style={{ fontSize: 17, fontWeight: 400, margin: 0 }}>Providers</h3>
        <button onClick={() => setShowAdd(true)}
                style={{ fontSize: 11, color: 'var(--ink-2)', padding: '4px 12px',
                         border: 'var(--hairline)', borderRadius: 4,
                         background: 'var(--paper)', cursor: 'pointer' }}>
          + Add provider
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {D.providers.map(p => (
          <ProviderCard key={p.id} provider={p}
                        isConfigured={configured[p.id]}
                        onConfigure={(key) => {
                          setKeys(k => ({ ...k, [p.id]: key }));
                          setConfigured(c => ({ ...c, [p.id]: true }));
                        }}
                        progress={progress}
                        pullQueue={pullQueue}
                        setPullQueue={setPullQueue}/>
        ))}
      </div>

      {showAdd && <AddProviderModal D={D}
                                    onAdd={() => setShowAdd(false)}
                                    onClose={() => setShowAdd(false)}/>}
    </>
  );
}

function ProviderCard({ provider, isConfigured, onConfigure,
                        progress, pullQueue, setPullQueue }) {
  const [open, setOpen] = iUseS(isConfigured);
  const [keyInput, setKeyInput] = iUseS("");

  const availableModels = provider.id === "ollama"
    ? provider.models.filter(m => m.pulled || pullQueue[m.id]).length
    : (isConfigured ? provider.models.length : 0);

  return (
    <div style={{ background: 'var(--paper)', border: 'var(--hairline)',
                   borderRadius: 6, overflow: 'hidden' }}>
      <button onClick={() => setOpen(o => !o)}
              style={{ width: '100%', display: 'grid',
                       gridTemplateColumns: '36px 1fr auto auto', gap: 12,
                       alignItems: 'center', padding: '12px 16px',
                       background: 'transparent', border: 'none',
                       cursor: 'pointer', textAlign: 'left' }}>
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)',
                                           textAlign: 'center' }}>{provider.kanji}</span>
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
            <span className="display" style={{ fontSize: 15 }}>{provider.name}</span>
            <span style={{ fontSize: 11, letterSpacing: '0.1em', textTransform: 'uppercase',
                            color: 'var(--ink-4)' }}>
              {provider.kind === "local" ? "local · ollama" : "cloud"}
            </span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
            {provider.note}
          </div>
        </div>
        <div>
          {isConfigured ? (
            <span style={{ fontSize: 11, color: 'var(--success)',
                            padding: '4px 8px', borderRadius: 3,
                            background: 'rgba(122,158,98,.10)',
                            letterSpacing: '0.08em', textTransform: 'uppercase',
                            display: 'inline-flex', alignItems: 'center', gap: 4 }}>
              ✓ {availableModels} of {provider.models.length} model{provider.models.length !== 1 && "s"}
            </span>
          ) : (
            <span style={{ fontSize: 11, color: 'var(--ink-3)',
                            padding: '4px 8px', borderRadius: 3,
                            background: 'var(--paper-2)',
                            letterSpacing: '0.08em', textTransform: 'uppercase' }}>
              not configured
            </span>
          )}
        </div>
        <span style={{ fontSize: 11, color: 'var(--ink-4)',
                        transform: open ? 'rotate(90deg)' : 'none',
                        transition: 'transform .15s', display: 'inline-block' }}>▶</span>
      </button>

      {open && (
        <div style={{ borderTop: 'var(--hairline)', padding: '12px 16px 16px',
                       background: 'var(--paper-2)' }}>
          {!isConfigured && provider.envVar && (
            <KeyInput envVar={provider.envVar} value={keyInput} onChange={setKeyInput}
                      onSave={() => onConfigure(keyInput)}/>
          )}

          {provider.id === "ollama" ? (
            <OllamaModelTable models={provider.models}
                              progress={progress}
                              pullQueue={pullQueue}
                              setPullQueue={setPullQueue}/>
          ) : (
            <CloudModelTable models={provider.models}
                             isConfigured={isConfigured}/>
          )}
        </div>
      )}
    </div>
  );
}
