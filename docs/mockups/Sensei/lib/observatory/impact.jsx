// Change impact reports — closes the loop on accepted recommendations.
//
// Two screens in one file:
//   ▸ ImpactReports — list of all reports + detail panel; verdict-coloured.
//   ▸ NegativeImpactAlert — the stand-alone red flag for a negative report.

const { useState: ciS } = React;

const VERDICT_META = {
  positive: { glyph: "好", color: "var(--success)",  label: "positive impact" },
  neutral:  { glyph: "並", color: "var(--ink-3)", label: "no measurable effect" },
  negative: { glyph: "悪", color: "var(--accent)",   label: "negative impact" }
};

// ═══════════════════════════════════════════════════════════════════════
// SCREEN A · Change Impact Report (full list + detail)
// ═══════════════════════════════════════════════════════════════════════
function ObsImpact({ state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="果"
    emptyTitle="No impact to measure yet"
    emptyHint="Accept a recommendation and sensei measures before/after — the verdict lands here once there's enough session data."
    errorHint="Couldn't load impact reports. Try again." onRetry={() => {}} />;
  const reports = window.UPGRADES.impactReports;
  const [openId, setOpen] = ciS(reports[0].id);
  const r = reports.find(x => x.id === openId) || reports[0];

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Change impact"
 >

      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>果</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Observatory · Change impact
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            Did sensei's advice actually work?
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            Each accepted recommendation gets a measurement window. FTR,
            corrections, tool usage and session duration are compared
            before vs after. The MOE panel writes the reasoning.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <UgMini n={reports.filter(r => r.verdict === "positive").length} l="positive" accent/>
          <UgMini n={reports.filter(r => r.verdict === "neutral").length} l="neutral"/>
          <UgMini n={reports.filter(r => r.verdict === "negative").length} l="negative"/>
        </div>
      </div>

      <div className="flex-1 grid min-h-0" style={{
 gridTemplateColumns: '300px 1fr' }}>
        <aside className="py-2 px-0 border-r overflow-auto" >
          {reports.map(rr => {
            const vm = VERDICT_META[rr.verdict];
            const open = openId === rr.id;
            return (
              <button key={rr.id} onClick={() => setOpen(rr.id)}
 style={{
 background: open ? 'var(--paper-2)' : 'transparent',
 borderLeft: open ? `2px solid ${vm.color}`
 : '2px solid transparent' }} className="py-3 px-4 w-full text-left cursor-pointer" >
                <div className="gap-2 flex items-center" >
                  <span className="kanji" style={{ fontSize: 13, color: vm.color }}>{vm.glyph}</span>
                  <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.14em', color: vm.color }}>{rr.verdict}</span>
                  <span className="flex-1" />
                  <span className="mono" style={{ fontSize: 11,
                                color: rr.ftrDelta >= 0 ? 'var(--success)' : 'var(--accent)' }}>
                    {rr.ftrDelta >= 0 ? "+" : ""}{Math.round(rr.ftrDelta*100)}%
                  </span>
                </div>
                <div style={{
 fontSize: 13,
 color: open ? 'var(--ink)' : 'var(--ink-2)',
 lineHeight: 1.4 }} className="mt-1 font-medium" >
                  {rr.title}
                </div>
                <div className="mono mt-1 text-ink-4" style={{
 fontSize: 11 }}>
                  {rr.window}
                </div>
              </button>
            );
          })}
        </aside>

        <main className="pt-6 pb-8 px-12 overflow-auto" >
          <ImpactDetail r={r}/>
        </main>
      </div>
    </div>
  );
}

function ImpactDetail({ r }) {
  const vm = VERDICT_META[r.verdict];

  return (
    <div style={{ maxWidth: 800 }}>
      {/* Eyebrow */}
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="gap-3 mb-3 flex items-center text-ink-3 uppercase" >
        <span className="mono" style={{ letterSpacing: 0 }}>{r.recId}</span>
        <Sep/>
        <span className="mono" style={{ letterSpacing: 0 }}>{r.project}</span>
        <Sep/>
        <span>acted {r.acted}</span>
        <Sep/>
        <span>measured {r.measured}</span>
      </div>

      <h2 className="display mt-0 mb-4 font-light text-ink" style={{
 fontSize: 28,
 lineHeight: 1.2, letterSpacing: '-0.015em' }}>
        {r.title}
      </h2>

      {/* Verdict pill + window */}
      <div className="gap-3 mb-6 flex items-center" >
        <div style={{
 background: `color-mix(in oklab, ${vm.color}, transparent 85%)`,
 borderRadius: 18,
 border: `1px solid color-mix(in oklab, ${vm.color}, transparent 70%)`
 }} className="gap-2 py-1 px-3 inline-flex items-center" >
          <span className="kanji" style={{ fontSize: 13, color: vm.color }}>{vm.glyph}</span>
          <span className="uppercase font-medium" style={{ fontSize: 11, color: vm.color,
 letterSpacing: '0.14em' }}>{vm.label}</span>
        </div>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {r.window}
        </span>
      </div>

      {/* Before / after metric grid */}
      <div style={{ gridTemplateColumns: 'repeat(4, 1fr)', background: 'var(--edge)',
 borderRadius: 6 }} className="gap-1 mb-6 grid overflow-hidden" >
        <BeforeAfter label="First-Try-Right"
                     before={`${Math.round(r.baselineFtr*100)}%`}
                     after={`${Math.round(r.currentFtr*100)}%`}
                     delta={`${r.ftrDelta >= 0 ? "+" : ""}${Math.round(r.ftrDelta*100)}pp`}
                     positive={r.ftrDelta >= 0}/>
        <BeforeAfter label="Corrections / session"
                     before={r.baselineCorrections.toFixed(1)}
                     after={r.currentCorrections.toFixed(1)}
                     delta={`${r.correctionsDelta >= 0 ? "+" : ""}${r.correctionsDelta.toFixed(1)}`}
                     positive={r.correctionsDelta <= 0}/>
        <BeforeAfter label="Avg session"
                     before="—"
                     after={`${r.avgSessionDelta >= 0 ? "+" : ""}${r.avgSessionDelta} min`}
                     delta=""
                     positive={r.avgSessionDelta <= 0}/>
        <BeforeAfter label="Tool-usage shift"
                     before={Object.keys(r.toolUsageDelta).length === 0 ? "—" :
                             Object.keys(r.toolUsageDelta).length + " tools"}
                     after=""
                     delta=""
                     positive={true}/>
      </div>

      {/* Tool usage detail */}
      {Object.keys(r.toolUsageDelta).length > 0 && (
        <div className="mb-6" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >Tool usage delta</div>
          <div className="gap-1 flex flex-col" >
            {Object.entries(r.toolUsageDelta).map(([tool, d]) => (
              <div key={tool} style={{ borderRadius: 4
 }} className="gap-2 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
                <span className="mono text-ink flex-1" style={{ fontSize: 11 }}>{tool}</span>
                <ToolBar value={d}/>
                <span className="mono text-right" style={{ fontSize: 11,
 color: d >= 0 ? 'var(--success)' : 'var(--accent)',
 minWidth: 48 }}>
                  {d >= 0 ? "+" : ""}{d}%
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* MOE panel */}
      <div style={{
 borderLeft: `2px solid ${vm.color}`,
 borderRadius: 6
 }} className="py-4 px-6 mb-6 bg-paper-2 border border-paper-edge" >
        <div className="gap-2 mb-2 flex items-center" >
          <span className="kanji text-accent" style={{ fontSize: 13 }}>議</span>
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>MOE panel reasoning</span>
          <span className="flex-1" />
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            {r.moeReasoning.consensus}
          </span>
        </div>
        <div style={{
 fontSize: 13, lineHeight: 1.5 }} className="mb-2 text-ink font-medium" >
          {r.moeReasoning.headline}
        </div>
        <p style={{
 fontSize: 13, lineHeight: 1.65
 }} className="mt-0 mb-3 text-ink-2" >{r.moeReasoning.body}</p>

        {/* Per-model votes */}
        <div className="gap-1 pt-3 flex flex-col border-t" >
          {r.moeReasoning.models.map((m, i) => {
            const mv = VERDICT_META[m.verdict];
            return (
              <div key={i} style={{
 gridTemplateColumns: '120px 14px 1fr' }} className="gap-2 grid items-start" >
                <span className="mono text-ink" style={{ fontSize: 11 }}>
                  {m.name}
                </span>
                <span className="kanji mt-1" style={{
 fontSize: 13, color: mv.color
}}>{mv.glyph}</span>
                <span className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.5 }}>
                  {m.note}
                </span>
              </div>
            );
          })}
        </div>

        {r.moeReasoning.suggestedRevision && (
          <div style={{ borderRadius: 4 }} className="mt-3 py-2 px-3 bg-paper border border-paper-edge" >
            <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-accent uppercase" >
              Suggested revision
            </div>
            <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.55 }}>
              {r.moeReasoning.suggestedRevision}
            </div>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="gap-2 pt-1 flex items-center" >
        {r.verdict === "negative" ? (
          <>
            <button style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-ink text-paper border-0 cursor-pointer inline-flex items-center" >
              <span className="kanji text-accent" style={{ fontSize: 13 }}>戻</span>
              Revert change
            </button>
            <FlatBtn glyph="改" label="Revise the rule"/>
          </>
        ) : (
          <FlatBtn glyph="改" label="Revise"/>
        )}
        <FlatBtn glyph="観" label="Keep monitoring"/>
        <span className="flex-1" />
        <FlatBtn glyph="納" label="Dismiss" subtle/>
      </div>
    </div>
  );
}

function BeforeAfter({ label, before, after, delta, positive }) {
  return (
    <div className="py-3 px-4 bg-paper-2" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-4 uppercase" >{label}</div>
      <div className="gap-2 flex items-baseline" >
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{before}</span>
        {after && <span className="text-ink-4" style={{ fontSize: 11 }}>→</span>}
        {after && (
          <span className="display font-normal text-ink" style={{ fontSize: 17 }}>{after}</span>
        )}
      </div>
      {delta && (
        <div className="mono mt-1" style={{
 fontSize: 11,
                       color: positive ? 'var(--success)' : 'var(--accent)'
}}>{delta}</div>
      )}
    </div>
  );
}

function ToolBar({ value }) {
  const w = 80;
  const half = w / 2;
  const len = Math.min(half, Math.abs(value) / 50 * half);
  const positive = value >= 0;
  return (
    <svg width={w} height={8}>
      <line x1={half} y1={0} x2={half} y2={8} stroke="var(--edge)" strokeWidth="1"/>
      <rect x={positive ? half : half - len} y={2} width={len} height={4}
            fill={positive ? 'var(--success)' : 'var(--accent)'} rx={1}/>
    </svg>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// SCREEN B · Negative Impact Alert (focused on a single bad outcome)
// ═══════════════════════════════════════════════════════════════════════
function ObsNegativeAlert() {
  const reports = window.UPGRADES.impactReports;
  const r = reports.find(x => x.verdict === "negative") || reports[0];

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Negative impact alert"
 >

      {/* The alert banner */}
      <div className="py-3 px-8 gap-3 bg-accent flex items-center text-paper" >
        <span className="kanji" style={{ fontSize: 22 }}>警</span>
        <div className="flex-1" >
          <div className="uppercase" style={{ fontSize: 11, letterSpacing: '0.18em',
 opacity: 0.8 }}>regression detected</div>
          <div className="font-medium" style={{ fontSize: 13 }}>
            A change you accepted on {r.acted} is hurting your FTR.
            Sensei surfaced this for review.
          </div>
        </div>
        <button style={{
 fontSize: 11, borderRadius: 4 }} className="py-1 px-3 text-paper bg-on-primary-faint border-0 cursor-pointer" >
          dismiss
        </button>
      </div>

      <div style={{
 maxWidth: 920 }} className="py-8 px-16 mx-auto flex-1 overflow-auto w-full" >

        {/* Headline */}
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-accent uppercase font-medium" >
          Negative impact · {r.window}
        </div>
        <h1 className="display mt-0 mb-3 font-light text-ink" style={{
 fontSize: 28, lineHeight: 1.2,
 letterSpacing: '-0.015em' }}>
          {r.title}
        </h1>
        <p style={{ fontSize: 13, lineHeight: 1.65 }} className="mt-0 mb-6 text-ink-2" >
          {r.moeReasoning.headline}
        </p>

        {/* The two big deltas */}
        <div style={{ gridTemplateColumns: '1fr 1fr'
 }} className="gap-3 mb-6 grid" >
          <DeltaCard label="First-Try-Right"
                     before={`${Math.round(r.baselineFtr*100)}%`}
                     after={`${Math.round(r.currentFtr*100)}%`}
                     delta={`${Math.round(r.ftrDelta*100)}pp`}
                     dir="down" bad/>
          <DeltaCard label="Corrections / session"
                     before={r.baselineCorrections.toFixed(1)}
                     after={r.currentCorrections.toFixed(1)}
                     delta={`+${r.correctionsDelta.toFixed(1)}`}
                     dir="up" bad/>
        </div>

        {/* Why — MOE reasoning */}
        <div style={{
 borderRadius: 6
 }} className="py-4 px-6 mb-6 bg-paper-2 border border-paper-edge" >
          <div className="gap-2 mb-3 flex items-center" >
            <span className="kanji text-accent" style={{ fontSize: 13 }}>議</span>
            <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>Why · MOE panel</span>
            <span className="flex-1" />
            <span className="mono text-accent" style={{ fontSize: 11 }}>
              {r.moeReasoning.consensus}
            </span>
          </div>
          <p style={{
 fontSize: 13, lineHeight: 1.7
 }} className="mt-0 mb-4 text-ink" >{r.moeReasoning.body}</p>
          <div className="gap-2 pt-3 flex flex-col border-t" >
            {r.moeReasoning.models.map((m, i) => {
              const mv = VERDICT_META[m.verdict];
              return (
                <div key={i} style={{
 gridTemplateColumns: '130px 14px 1fr' }} className="gap-3 grid items-start" >
                  <span className="mono text-ink" style={{ fontSize: 11 }}>
                    {m.name}
                  </span>
                  <span className="kanji" style={{ fontSize: 13, color: mv.color }}>{mv.glyph}</span>
                  <span className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.55 }}>
                    {m.note}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Suggested revision */}
        {r.moeReasoning.suggestedRevision && (
          <div style={{ border: '1px solid var(--accent)',
 borderRadius: 6
 }} className="py-4 px-4 mb-6 bg-paper" >
            <div className="gap-2 mb-2 flex items-center" >
              <span className="kanji text-accent" style={{ fontSize: 13 }}>改</span>
              <span className="text-accent uppercase font-medium" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
                Recommended fix
              </span>
            </div>
            <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.6 }}>
              {r.moeReasoning.suggestedRevision}
            </div>
          </div>
        )}

        {/* Actions */}
        <div className="gap-2 flex items-center" >
          <button style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-ink text-paper border-0 cursor-pointer inline-flex items-center" >
            <span className="kanji text-accent" style={{ fontSize: 13 }}>戻</span>
            Revert change
          </button>
          <button style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-paper-2 text-ink border border-paper-edge cursor-pointer inline-flex items-center" >
            <span className="kanji text-accent" style={{ fontSize: 13 }}>改</span>
            Revise the rule
          </button>
          <FlatBtn glyph="観" label="Keep monitoring"/>
          <span className="flex-1" />
          <FlatBtn glyph="納" label="Dismiss" subtle/>
        </div>
      </div>
    </div>
  );
}

function DeltaCard({ label, before, after, delta, dir, bad }) {
  return (
    <div style={{
 borderRadius: 6
 }} className="py-4 px-6 bg-paper-2 border border-paper-edge" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-3 text-ink-3 uppercase" >{label}</div>
      <div className="gap-3 flex items-baseline" >
        <span className="display font-light text-ink-3" style={{ fontSize: 28,
 lineHeight: 1 }}>{before}</span>
        <span className="text-ink-4" style={{ fontSize: 13 }}>→</span>
        <span className="display font-light" style={{ fontSize: 40,
 color: bad ? 'var(--accent)' : 'var(--success)',
 lineHeight: 1 }}>{after}</span>
        <span className="flex-1" />
        <div style={{
 fontSize: 13, color: bad ? 'var(--accent)' : 'var(--success)' }} className="gap-1 inline-flex items-center font-medium" >
          <span style={{ fontSize: 13 }}>{dir === "down" ? "↓" : "↑"}</span>
          <span className="mono">{delta}</span>
        </div>
      </div>
    </div>
  );
}

window.ObsImpact = ObsImpact;
window.ObsNegativeAlert = ObsNegativeAlert;
