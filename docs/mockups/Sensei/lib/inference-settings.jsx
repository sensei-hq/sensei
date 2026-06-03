// Inference settings — local + provider models, fallback chain, MOE deliberation panel.
// Benchmark runner — corpus picker + A/B run results comparing with vs without sensei.

const { useState: ifS } = React;

// ─── Inference settings ────────────────────────────────────
function InferenceSettings() {
  const I = window.EXT_DATA.inference;
  const [tab, setTab] = ifS("models"); // models · routing · moe

  return (
    <div className="sensei" data-screen-label="Inference settings"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* Hero */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>智</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >Configure · Inference</div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            Where sensei thinks.
          </h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >
            Models live in a fallback chain — sensei tries each in order. For
            high-stakes calls, the panel deliberates: same question to N models,
            cross-critique, refine, converge.
          </p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex'
}} className="gap-5 pl-5" >
          <IfMini n={I.local.filter(m => m.pulled).length} l="local"/>
          <IfMini n={I.providers.filter(p => p.configured).length} l="providers" mono/>
          <IfMini n={I.moe.panelists.filter(p => p.online).length} l="panel" mono accent/>
        </div>
      </div>

      {/* Tabs */}
      <div style={{
 display: 'flex', borderBottom: 'var(--hairline)', background: 'var(--paper-2)'
}} className="px-6" >
        {[
          ["models",  "具", "Models"],
          ["routing", "路", "Routing & fallback"],
          ["moe",     "群", "MOE panel"],
        ].map(([id, kanji, label]) => (
          <button key={id} onClick={() => setTab(id)} style={{
 display: 'inline-flex', alignItems: 'center',
            borderBottom: tab === id ? '2px solid var(--accent)' : '2px solid transparent',
            background: 'transparent', color: tab === id ? 'var(--ink)' : 'var(--ink-3)',
            fontSize: 13, cursor: 'pointer', fontFamily: 'var(--font-ui)', border: 'none'
}} className="py-3 px-4 gap-2" >
            <span className="kanji" style={{ fontSize: 13,
              color: tab === id ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
            {label}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }} className="py-5 px-6" >
        {tab === "models" && <IfModelsTab I={I}/>}
        {tab === "routing" && <IfRoutingTab I={I}/>}
        {tab === "moe" && <IfMoeTab I={I}/>}
      </div>
    </div>
  );
}

function IfModelsTab({ I }) {
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-5" >
      {/* Local */}
      <section>
        <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>Local · Ollama</h3>
        <p style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-0 mb-3" >
          Models pulled to disk. Run offline. Privacy mode forces these.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {I.local.map(m => (
            <div key={m.id} style={{
 borderRadius: 5,
              background: m.pulled ? 'var(--paper-2)' : 'transparent',
              border: 'var(--hairline)',
              display: 'grid', gridTemplateColumns: '1fr auto auto',
              alignItems: 'center'
}} className="py-3 px-3 gap-3" >
              <div>
                <div className="mono" style={{ fontSize: 13,
                  color: m.pulled ? 'var(--ink)' : 'var(--ink-3)' }}>{m.id}</div>
                <div style={{
 display: 'flex',
                               fontSize: 11, color: 'var(--ink-4)'
}} className="gap-1 mt-1" >
                  <span className="mono">{m.size}</span>
                  {m.cap.reasoning > 0 && (
                    <span>· reasoning {dotsFor(m.cap.reasoning)}</span>
                  )}
                  {m.cap.code > 0 && <span>· code {dotsFor(m.cap.code)}</span>}
                  {m.cap.embed && <span style={{ color: 'var(--accent)' }}>· embeddings</span>}
                </div>
              </div>
              {m.default && (
                <span style={{ fontSize: 11, color: 'var(--accent)',
                                letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                  default
                </span>
              )}
              <span style={{ fontSize: 11,
                              color: m.pulled ? 'var(--success)' : 'var(--ink-3)',
                              letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                {m.status}
              </span>
            </div>
          ))}
          <button style={{
 fontSize: 11, color: 'var(--ink-3)',
            background: 'transparent', border: '1px dashed var(--edge)',
            borderRadius: 4, cursor: 'pointer'
}} className="p-2 mt-1" >
            + pull model
          </button>
        </div>
      </section>

      {/* Providers */}
      <section>
        <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>External providers</h3>
        <p style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-0 mb-3" >
          API keys live in your OS keychain — never in project files.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {I.providers.map(p => (
            <div key={p.id} style={{
 borderRadius: 5,
              background: p.configured ? 'var(--paper-2)' : 'transparent',
              border: 'var(--hairline)'
}} className="py-3 px-3" >
              <div style={{
 display: 'grid',
                             gridTemplateColumns: '1fr auto auto',
                             alignItems: 'center'
}} className="gap-3" >
                <span style={{ fontSize: 13, color: p.configured ? 'var(--ink)' : 'var(--ink-3)',
                                fontWeight: 500 }}>{p.label}</span>
                <span className="mono" style={{ fontSize: 11,
                  color: p.configured ? 'var(--ink-2)' : 'var(--ink-4)' }}>{p.keyMasked}</span>
                <span style={{ fontSize: 11,
                                color: p.configured ? 'var(--success)' : 'var(--ink-4)',
                                letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                  {p.lastTested}
                </span>
              </div>
              {p.configured && (
                <div style={{ display: 'flex', flexWrap: 'wrap' }} className="mt-2 gap-1" >
                  {p.models.map(mm => (
                    <span key={mm} className="mono py-1 px-2" style={{
 fontSize: 11,
                      color: 'var(--ink-3)', background: 'var(--paper)', borderRadius: 3,
                      border: 'var(--hairline)'
}}>{mm}</span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function IfRoutingTab({ I }) {
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1fr' }} className="gap-6" >
      {/* Fallback chain */}
      <section>
        <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>Fallback chain</h3>
        <p style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-0 mb-4" >
          Sensei tries A → B → C until one succeeds. Drag to reorder.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-0" >
          {I.fallbackChain.map((f, i) => (
            <div key={f.id} style={{
 position: 'relative',
              display: 'grid', gridTemplateColumns: '40px 1fr auto',
              alignItems: 'center',
              border: 'var(--hairline)', borderRadius: 6,
              background: 'var(--paper-2)'
}} className="gap-3 py-3 px-4 mb-2" >
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                <span style={{ width: 24, height: 24, borderRadius: '50%',
                  background: i === 0 ? 'var(--accent)' : 'var(--edge)',
                  color: i === 0 ? 'var(--paper)' : 'var(--ink-2)',
                  display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                  fontSize: 13, fontFamily: 'var(--font-mono)' }}>{i + 1}</span>
                {i < I.fallbackChain.length - 1 && (
                  <span style={{
 width: 1, height: 18, background: 'var(--edge)', marginBottom: -10
}} className="mt-1" />
                )}
              </div>
              <div>
                <div className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>
                  {f.model}
                </div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
                  via <span className="mono">{f.provider}</span>  ·  {f.reason}
                </div>
              </div>
              <button style={{ fontSize: 13, color: 'var(--ink-4)',
                                background: 'transparent', border: 'none',
                                cursor: 'grab' }}>⋮⋮</button>
            </div>
          ))}
          <button style={{
 fontSize: 11, color: 'var(--ink-3)',
            background: 'transparent', border: '1px dashed var(--edge)',
            borderRadius: 4, cursor: 'pointer'
}} className="p-2 mt-1" >
            + add fallback
          </button>
        </div>
      </section>

      {/* Per-task routing */}
      <section>
        <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>Per-task routing</h3>
        <p style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-0 mb-4" >
          Override the fallback for specific task types.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {I.routing.map(r => (
            <div key={r.task} style={{
              display: 'grid', gridTemplateColumns: '110px 1fr', alignItems: 'start',
              border: 'var(--hairline)', borderRadius: 5
}} className="gap-3 py-3 px-3" >
              <span style={{
 fontSize: 11, color: 'var(--ink-3)',
                              letterSpacing: '0.12em', textTransform: 'uppercase'
}} className="pt-1" >{r.task}</span>
              <div>
                <div className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>
                  {r.model}
                </div>
                <div style={{ fontSize: 11, color: 'var(--ink-4)' }} className="mt-1" >
                  {r.reason}
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function IfMoeTab({ I }) {
  const moe = I.moe;
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 1fr' }} className="gap-6" >
      <section>
        <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>Deliberation panel</h3>
        <p style={{
 fontSize: 11, color: 'var(--ink-3)',
                     lineHeight: 1.55, maxWidth: 520
}} className="mt-0 mb-4" >
          Same input goes to every panelist. They draft independently, then
          cross-critique each other's answers, then refine. After {moe.cycles} cycles
          the verdicts are reconciled.
        </p>

        <div className="mb-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                         textTransform: 'uppercase'
}} className="mb-2" >Panelists</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {moe.panelists.map(p => (
              <div key={p.id} style={{
                display: 'grid', gridTemplateColumns: '8px 1fr auto auto', alignItems: 'center',
                background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 5
}} className="gap-3 py-3 px-3" >
                <span style={{ width: 8, height: 8, borderRadius: '50%',
                  background: p.online ? 'var(--success)' : 'var(--ink-4)' }}/>
                <div>
                  <div className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>{p.label}</div>
                  <div style={{ fontSize: 11, color: 'var(--ink-4)' }} className="mt-1" >
                    {p.role}
                  </div>
                </div>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  weight {p.weight.toFixed(1)}
                </span>
                <button style={{ fontSize: 13, color: 'var(--ink-4)',
                                  background: 'transparent', border: 'none',
                                  cursor: 'pointer' }}>×</button>
              </div>
            ))}
            <button style={{
 fontSize: 11, color: 'var(--ink-3)',
              background: 'transparent', border: '1px dashed var(--edge)',
              borderRadius: 4, cursor: 'pointer'
}} className="p-2" >
              + add panelist
            </button>
          </div>
        </div>

        {/* Cycles diagram */}
        <div style={{
 borderRadius: 5,
                       background: 'var(--paper-2)', border: 'var(--hairline)'
}} className="py-4 px-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                         textTransform: 'uppercase'
}} className="mb-3" >Strategy</div>
          <div className="mono" style={{ fontSize: 13, color: 'var(--ink-2)',
                                          lineHeight: 1.7 }}>
            {moe.strategy.split(' → ').map((step, i, arr) => (
              <React.Fragment key={i}>
                <span style={{ color: 'var(--ink)' }}>{step}</span>
                {i < arr.length - 1 && <span style={{ color: 'var(--accent)' }}> → </span>}
              </React.Fragment>
            ))}
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-2" >
            <span className="mono">{moe.cycles} cycles</span>  ·
            converges when {moe.panelists.length} panelists agree above 0.80.
          </div>
        </div>
      </section>

      <section>
        <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>When to use it</h3>
        <p style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-0 mb-3" >
          MOE is expensive. Reserve it for high-stakes calls.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {moe.whenToUse.map(w => (
            <div key={w} style={{
              background: 'var(--paper-2)', border: 'var(--hairline)',
              borderRadius: 5, fontSize: 13, color: 'var(--ink-2)'
}} className="py-2 px-3" >{w}</div>
          ))}
        </div>

        {/* Last run */}
        <div style={{
 borderRadius: 6,
                       border: 'var(--hairline)', background: 'var(--paper-2)'
}} className="mt-5 py-4 px-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                         textTransform: 'uppercase'
}} className="mb-2" >Most recent run</div>
          <div style={{
 fontSize: 13, color: 'var(--ink)', lineHeight: 1.5
}} className="mb-2" >"{moe.lastRun.topic}"</div>
          <div style={{
 display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
                         fontSize: 11
}} className="mb-2" >
            <span style={{ color: 'var(--ink-4)' }}>Duration</span>
            <span className="mono" style={{ color: 'var(--ink-2)' }}>
              {(moe.lastRun.durationMs/1000).toFixed(1)}s
            </span>
            <span style={{ color: 'var(--ink-4)' }}>Agreement</span>
            <span className="mono" style={{ color: 'var(--accent)' }}>
              {(moe.lastRun.agreement*100).toFixed(0)}%
            </span>
          </div>
          <div style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55, background: 'var(--paper)',
                         borderRadius: 4, borderLeft: '2px solid var(--accent)'
}} className="py-2 px-3" >
            <span style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--accent)',
                            textTransform: 'uppercase', display: 'block'
}} className="mb-1" >
              Verdict
            </span>
            {moe.lastRun.verdict}
          </div>
        </div>
      </section>
    </div>
  );
}

function dotsFor(n) {
  return Array.from({ length: 5 }, (_, i) =>
    i < n ? '●' : '○').join('');
}
function IfMini({ n, l, mono, accent }) {
  return (
    <div style={{ textAlign: 'right' }}>
      <div className={mono ? "mono" : "display"} style={{
        fontSize: mono ? 13 : 22, color: accent ? 'var(--accent)' : 'var(--ink)',
        fontWeight: 400, lineHeight: 1
      }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase'
}} className="mt-1" >{l}</div>
    </div>
  );
}

window.InferenceSettings = InferenceSettings;
