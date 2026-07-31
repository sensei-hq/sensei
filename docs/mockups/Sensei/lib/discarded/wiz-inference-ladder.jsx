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
    <div className="shrink-0" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 uppercase text-ink-4 text-right" >variant</div>
      <div style={{
 borderRadius: 5 }} className="p-1 gap-0 flex bg-paper-2 border border-paper-edge" >
        {options.map(v => (
          <button key={v.id} onClick={() => onChange(v.id)}
 style={{
 fontSize: 11, borderRadius: 3,
 background: variant === v.id ? 'var(--paper)' : 'transparent',
 color: variant === v.id ? 'var(--ink)' : 'var(--ink-3)', letterSpacing: '0.04em'
 }} className="py-1 px-3 border-0 cursor-pointer" >
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
      <div className="mb-2 flex items-baseline justify-between" >
        <h3 className="display m-0 font-normal" style={{ fontSize: 17 }}>Providers</h3>
        <button onClick={() => setShowAdd(true)}
 style={{
 fontSize: 11, borderRadius: 4 }} className="py-1 px-3 text-ink-2 border border-paper-edge bg-paper cursor-pointer" >
          + Add provider
        </button>
      </div>

      <div className="gap-2 flex flex-col" >
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
    <div className="bg-paper border border-paper-edge overflow-hidden" style={{
 borderRadius: 6 }}>
      <button onClick={() => setOpen(o => !o)}
 style={{
 gridTemplateColumns: '36px 1fr auto auto' }} className="gap-3 py-3 px-4 w-full grid items-center bg-transparent border-0 cursor-pointer text-left" >
        <span className="kanji text-accent text-center" style={{ fontSize: 22 }}>{provider.kanji}</span>
        <div>
          <div className="gap-2 flex items-baseline" >
            <span className="display" style={{ fontSize: 15 }}>{provider.name}</span>
            <span className="uppercase text-ink-4" style={{ fontSize: 11, letterSpacing: '0.1em' }}>
              {provider.kind === "local" ? "local · ollama" : "cloud"}
            </span>
          </div>
          <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
            {provider.note}
          </div>
        </div>
        <div>
          {isConfigured ? (
            <span style={{
 fontSize: 11, borderRadius: 3,
 background: 'rgba(122,158,98,.10)',
 letterSpacing: '0.08em' }} className="py-1 px-2 gap-1 text-success uppercase inline-flex items-center" >
              ✓ {availableModels} of {provider.models.length} model{provider.models.length !== 1 && "s"}
            </span>
          ) : (
            <span style={{
 fontSize: 11, borderRadius: 3,
 letterSpacing: '0.08em' }} className="py-1 px-2 text-ink-3 bg-paper-2 uppercase" >
              not configured
            </span>
          )}
        </div>
        <span className="text-ink-4 inline-block" style={{ fontSize: 11,
 transform: open ? 'rotate(90deg)' : 'none',
 transition: 'transform .15s' }}>▶</span>
      </button>

      {open && (
        <div className="pt-3 pb-4 px-4 border-t bg-paper-2" >
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
