// Inference settings — local + provider models, fallback chain, MOE deliberation panel.
// Benchmark runner — corpus picker + A/B run results comparing with vs without sensei.

const { useState: ifS } = React;

// ─── Inference settings ────────────────────────────────────
function InferenceSettings() {
  const I = window.EXT_DATA.inference;
  const [tab, setTab] = ifS("models"); // models · routing · moe

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Inference settings"
 >

      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>智</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >Configure · Inference</div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            Where sensei thinks.
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            Models live in a fallback chain — sensei tries each in order. For
            high-stakes calls, the panel deliberates: same question to N models,
            cross-critique, refine, converge.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <IfMini n={I.local.filter(m => m.pulled).length} l="local"/>
          <IfMini n={I.providers.filter(p => p.configured).length} l="providers" mono/>
          <IfMini n={I.moe.panelists.filter(p => p.online).length} l="panel" mono accent/>
        </div>
      </div>

      {/* Tabs */}
      <div className="px-8 flex border-b bg-paper-2" >
        {[
          ["models",  "具", "Models"],
          ["routing", "路", "Routing & fallback"],
          ["moe",     "群", "MOE panel"],
        ].map(([id, kanji, label]) => (
          <button key={id} onClick={() => setTab(id)} style={{
 borderBottom: tab === id ? '2px solid var(--accent)' : '2px solid transparent', color: tab === id ? 'var(--ink)' : 'var(--ink-3)',
 fontSize: 13, fontFamily: 'var(--font-ui)' }} className="py-3 px-4 gap-2 inline-flex items-center bg-transparent cursor-pointer border-0" >
            <span className="kanji" style={{ fontSize: 13,
              color: tab === id ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
            {label}
          </button>
        ))}
      </div>

      <div className="py-6 px-8 flex-1 min-h-0 overflow-auto" >
        {tab === "models" && <IfModelsTab I={I}/>}
        {tab === "routing" && <IfRoutingTab I={I}/>}
        {tab === "moe" && <IfMoeTab I={I}/>}
      </div>
    </div>
  );
}

function IfModelsTab({ I }) {
  return (
    <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-6 grid" >
      {/* Local */}
      <section>
        <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>Local · Ollama</h3>
        <p style={{ fontSize: 11 }} className="mt-0 mb-3 text-ink-3" >
          Models pulled to disk. Run offline. Privacy mode forces these.
        </p>
        <div className="gap-1 flex flex-col" >
          {I.local.map(m => (
            <div key={m.id} style={{
 borderRadius: 5,
 background: m.pulled ? 'var(--paper-2)' : 'transparent', gridTemplateColumns: '1fr auto auto' }} className="py-3 px-3 gap-3 border border-paper-edge grid items-center" >
              <div>
                <div className="mono" style={{ fontSize: 13,
                  color: m.pulled ? 'var(--ink)' : 'var(--ink-3)' }}>{m.id}</div>
                <div style={{
 fontSize: 11 }} className="gap-1 mt-1 flex text-ink-4" >
                  <span className="mono">{m.size}</span>
                  {m.cap.reasoning > 0 && (
                    <span>· reasoning {dotsFor(m.cap.reasoning)}</span>
                  )}
                  {m.cap.code > 0 && <span>· code {dotsFor(m.cap.code)}</span>}
                  {m.cap.embed && <span className="text-accent" >· embeddings</span>}
                </div>
              </div>
              {m.default && (
                <span className="text-accent uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
                  default
                </span>
              )}
              <span className="uppercase" style={{ fontSize: 11,
 color: m.pulled ? 'var(--success)' : 'var(--ink-3)',
 letterSpacing: '0.12em' }}>
                {m.status}
              </span>
            </div>
          ))}
          <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 mt-1 text-ink-3 bg-transparent cursor-pointer" >
            + pull model
          </button>
        </div>
      </section>

      {/* Providers */}
      <section>
        <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>External providers</h3>
        <p style={{ fontSize: 11 }} className="mt-0 mb-3 text-ink-3" >
          API keys live in your OS keychain — never in project files.
        </p>
        <div className="gap-1 flex flex-col" >
          {I.providers.map(p => (
            <div key={p.id} style={{
 borderRadius: 5,
 background: p.configured ? 'var(--paper-2)' : 'transparent' }} className="py-3 px-3 border border-paper-edge" >
              <div style={{
 gridTemplateColumns: '1fr auto auto' }} className="gap-3 grid items-center" >
                <span className="font-medium" style={{ fontSize: 13, color: p.configured ? 'var(--ink)' : 'var(--ink-3)' }}>{p.label}</span>
                <span className="mono" style={{ fontSize: 11,
                  color: p.configured ? 'var(--ink-2)' : 'var(--ink-4)' }}>{p.keyMasked}</span>
                <span className="uppercase" style={{ fontSize: 11,
 color: p.configured ? 'var(--success)' : 'var(--ink-4)',
 letterSpacing: '0.12em' }}>
                  {p.lastTested}
                </span>
              </div>
              {p.configured && (
                <div className="mt-2 gap-1 flex flex-wrap" >
                  {p.models.map(mm => (
                    <span key={mm} className="mono py-1 px-2 text-ink-3 bg-paper border border-paper-edge" style={{
 fontSize: 11, borderRadius: 3 }}>{mm}</span>
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
    <div style={{ gridTemplateColumns: '1.2fr 1fr' }} className="gap-8 grid" >
      {/* Fallback chain */}
      <section>
        <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>Fallback chain</h3>
        <p style={{ fontSize: 11 }} className="mt-0 mb-4 text-ink-3" >
          Sensei tries A → B → C until one succeeds. Drag to reorder.
        </p>
        <div className="gap-0 flex flex-col" >
          {I.fallbackChain.map((f, i) => (
            <div key={f.id} style={{ gridTemplateColumns: '40px 1fr auto', borderRadius: 6 }} className="gap-3 py-3 px-4 mb-2 relative grid items-center border border-paper-edge bg-paper-2" >
              <div className="flex flex-col items-center" >
                <span className="rounded-full inline-flex items-center justify-center" style={{ width: 24, height: 24,
 background: i === 0 ? 'var(--accent)' : 'var(--edge)',
 color: i === 0 ? 'var(--paper)' : 'var(--ink-2)',
 fontSize: 13, fontFamily: 'var(--font-mono)' }}>{i + 1}</span>
                {i < I.fallbackChain.length - 1 && (
                  <span style={{
 width: 1, height: 18, background: 'var(--edge)', marginBottom: -10
}} className="mt-1" />
                )}
              </div>
              <div>
                <div className="mono text-ink" style={{ fontSize: 13 }}>
                  {f.model}
                </div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
                  via <span className="mono">{f.provider}</span>  ·  {f.reason}
                </div>
              </div>
              <button className="text-ink-4 bg-transparent border-0" style={{ fontSize: 13,
 cursor: 'grab' }}>⋮⋮</button>
            </div>
          ))}
          <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 mt-1 text-ink-3 bg-transparent cursor-pointer" >
            + add fallback
          </button>
        </div>
      </section>

      {/* Per-task routing */}
      <section>
        <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>Per-task routing</h3>
        <p style={{ fontSize: 11 }} className="mt-0 mb-4 text-ink-3" >
          Override the fallback for specific task types.
        </p>
        <div className="gap-1 flex flex-col" >
          {I.routing.map(r => (
            <div key={r.task} style={{ gridTemplateColumns: '110px 1fr', borderRadius: 5
 }} className="gap-3 py-3 px-3 grid items-start border border-paper-edge" >
              <span style={{
 fontSize: 11,
 letterSpacing: '0.12em' }} className="pt-1 text-ink-3 uppercase" >{r.task}</span>
              <div>
                <div className="mono text-ink" style={{ fontSize: 13 }}>
                  {r.model}
                </div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
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
    <div style={{ gridTemplateColumns: '1.1fr 1fr' }} className="gap-8 grid" >
      <section>
        <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>Deliberation panel</h3>
        <p style={{
 fontSize: 11,
 lineHeight: 1.55, maxWidth: 520
 }} className="mt-0 mb-4 text-ink-3" >
          Same input goes to every panelist. They draft independently, then
          cross-critique each other's answers, then refine. After {moe.cycles} cycles
          the verdicts are reconciled.
        </p>

        <div className="mb-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >Panelists</div>
          <div className="gap-1 flex flex-col" >
            {moe.panelists.map(p => (
              <div key={p.id} style={{ gridTemplateColumns: '8px 1fr auto auto', borderRadius: 5
 }} className="gap-3 py-3 px-3 grid items-center bg-paper-2 border border-paper-edge" >
                <span className="rounded-full" style={{ width: 8, height: 8,
 background: p.online ? 'var(--success)' : 'var(--ink-4)' }}/>
                <div>
                  <div className="mono text-ink" style={{ fontSize: 13 }}>{p.label}</div>
                  <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
                    {p.role}
                  </div>
                </div>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                  weight {p.weight.toFixed(1)}
                </span>
                <button className="text-ink-4 bg-transparent border-0 cursor-pointer" style={{ fontSize: 13 }}>×</button>
              </div>
            ))}
            <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 text-ink-3 bg-transparent cursor-pointer" >
              + add panelist
            </button>
          </div>
        </div>

        {/* Cycles diagram */}
        <div style={{
 borderRadius: 5 }} className="py-4 px-4 bg-paper-2 border border-paper-edge" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-3 text-ink-4 uppercase" >Strategy</div>
          <div className="mono text-ink-2" style={{ fontSize: 13,
 lineHeight: 1.7 }}>
            {moe.strategy.split(' → ').map((step, i, arr) => (
              <React.Fragment key={i}>
                <span className="text-ink" >{step}</span>
                {i < arr.length - 1 && <span className="text-accent" > → </span>}
              </React.Fragment>
            ))}
          </div>
          <div style={{ fontSize: 11 }} className="mt-2 text-ink-3" >
            <span className="mono">{moe.cycles} cycles</span>  ·
            converges when {moe.panelists.length} panelists agree above 0.80.
          </div>
        </div>
      </section>

      <section>
        <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>When to use it</h3>
        <p style={{ fontSize: 11 }} className="mt-0 mb-3 text-ink-3" >
          MOE is expensive. Reserve it for high-stakes calls.
        </p>
        <div className="gap-1 flex flex-col" >
          {moe.whenToUse.map(w => (
            <div key={w} style={{
 borderRadius: 5, fontSize: 13 }} className="py-2 px-3 bg-paper-2 border border-paper-edge text-ink-2" >{w}</div>
          ))}
        </div>

        {/* Last run */}
        <div style={{
 borderRadius: 6 }} className="mt-6 py-4 px-4 border border-paper-edge bg-paper-2" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >Most recent run</div>
          <div style={{
 fontSize: 13, lineHeight: 1.5
 }} className="mb-2 text-ink" >"{moe.lastRun.topic}"</div>
          <div style={{ gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
 fontSize: 11
 }} className="mb-2 grid" >
            <span className="text-ink-4" >Duration</span>
            <span className="mono text-ink-2" >
              {(moe.lastRun.durationMs/1000).toFixed(1)}s
            </span>
            <span className="text-ink-4" >Agreement</span>
            <span className="mono text-accent" >
              {(moe.lastRun.agreement*100).toFixed(0)}%
            </span>
          </div>
          <div style={{
 fontSize: 13, lineHeight: 1.55,
 borderRadius: 4, borderLeft: '2px solid var(--accent)'
 }} className="py-2 px-3 text-ink-2 bg-paper" >
            <span style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-accent uppercase block" >
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
    <div className="text-right" >
      <div className={mono ? "mono" : "display"} style={{
        fontSize: mono ? 13 : 22, color: accent ? 'var(--accent)' : 'var(--ink)',
        fontWeight: 400, lineHeight: 1
      }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 text-ink-4 uppercase" >{l}</div>
    </div>
  );
}

window.InferenceSettings = InferenceSettings;
