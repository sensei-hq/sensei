// Change impact reports — closes the loop on accepted recommendations.
//
// Two screens in one file:
//   ▸ ImpactReports — list of all reports + detail panel; verdict-coloured.
//   ▸ NegativeImpactAlert — the stand-alone red flag for a negative report.

const { useState: ciS } = React;

const VERDICT_META = {
  positive: { glyph: "好", color: "var(--jade)",  label: "positive impact" },
  neutral:  { glyph: "並", color: "var(--sumi-3)", label: "no measurable effect" },
  negative: { glyph: "悪", color: "var(--shu)",   label: "negative impact" }
};

// ═══════════════════════════════════════════════════════════════════════
// SCREEN A · Change Impact Report (full list + detail)
// ═══════════════════════════════════════════════════════════════════════
function ObsImpact() {
  const reports = window.UPGRADES.impactReports;
  const [openId, setOpen] = ciS(reports[0].id);
  const r = reports.find(x => x.id === openId) || reports[0];

  return (
    <div className="sensei" data-screen-label="Observatory · Change impact"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      <div style={{ padding: '22px 36px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 22 }}>
        <div className="kanji" style={{ fontSize: 42, color: 'var(--shu)', lineHeight: 1 }}>果</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 5 }}>
            Observatory · Change impact
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--sumi)' }}>
            Did sensei's advice actually work?
          </h1>
          <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            Each accepted recommendation gets a measurement window. FTR,
            corrections, tool usage and session duration are compared
            before vs after. The MOE panel writes the reasoning.
          </p>
        </div>
        <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 22 }}>
          <UgMini n={reports.filter(r => r.verdict === "positive").length} l="positive" accent/>
          <UgMini n={reports.filter(r => r.verdict === "neutral").length} l="neutral"/>
          <UgMini n={reports.filter(r => r.verdict === "negative").length} l="negative"/>
        </div>
      </div>

      <div style={{ flex: 1, display: 'grid',
                     gridTemplateColumns: '300px 1fr',
                     minHeight: 0 }}>
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto', padding: '8px 0' }}>
          {reports.map(rr => {
            const vm = VERDICT_META[rr.verdict];
            const open = openId === rr.id;
            return (
              <button key={rr.id} onClick={() => setOpen(rr.id)}
                      style={{ width: '100%', textAlign: 'left',
                                padding: '12px 18px',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? `2px solid ${vm.color}`
                                                  : '2px solid transparent',
                                cursor: 'pointer' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span className="kanji" style={{ fontSize: 13, color: vm.color }}>{vm.glyph}</span>
                  <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: vm.color,
                                  textTransform: 'uppercase' }}>{rr.verdict}</span>
                  <span style={{ flex: 1 }}/>
                  <span className="mono" style={{ fontSize: 10.5,
                                color: rr.ftrDelta >= 0 ? 'var(--jade)' : 'var(--shu)' }}>
                    {rr.ftrDelta >= 0 ? "+" : ""}{Math.round(rr.ftrDelta*100)}%
                  </span>
                </div>
                <div style={{ fontSize: 12.5,
                               color: open ? 'var(--sumi)' : 'var(--sumi-2)',
                               lineHeight: 1.4, marginTop: 6, fontWeight: 500 }}>
                  {rr.title}
                </div>
                <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                              marginTop: 4 }}>
                  {rr.window}
                </div>
              </button>
            );
          })}
        </aside>

        <main style={{ overflow: 'auto', padding: '28px 44px 36px' }}>
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
      <div style={{ display: 'flex', alignItems: 'center', gap: 12,
                     fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase', marginBottom: 12 }}>
        <span className="mono" style={{ letterSpacing: 0 }}>{r.recId}</span>
        <Sep/>
        <span className="mono" style={{ letterSpacing: 0 }}>{r.project}</span>
        <Sep/>
        <span>acted {r.acted}</span>
        <Sep/>
        <span>measured {r.measured}</span>
      </div>

      <h2 className="display" style={{ fontSize: 28, fontWeight: 300,
                                        lineHeight: 1.2, letterSpacing: '-0.015em',
                                        margin: '0 0 18px', color: 'var(--sumi)' }}>
        {r.title}
      </h2>

      {/* Verdict pill + window */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 14,
                     marginBottom: 22 }}>
        <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8,
                       padding: '6px 12px',
                       background: `color-mix(in oklab, ${vm.color}, transparent 85%)`,
                       borderRadius: 18,
                       border: `1px solid color-mix(in oklab, ${vm.color}, transparent 70%)` }}>
          <span className="kanji" style={{ fontSize: 13, color: vm.color }}>{vm.glyph}</span>
          <span style={{ fontSize: 11, color: vm.color,
                          letterSpacing: '0.14em', textTransform: 'uppercase',
                          fontWeight: 500 }}>{vm.label}</span>
        </div>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
          {r.window}
        </span>
      </div>

      {/* Before / after metric grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                     gap: 1, background: 'var(--paper-edge)',
                     borderRadius: 6, overflow: 'hidden', marginBottom: 22 }}>
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
        <div style={{ marginBottom: 24 }}>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 10 }}>Tool usage delta</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {Object.entries(r.toolUsageDelta).map(([tool, d]) => (
              <div key={tool} style={{ display: 'flex', alignItems: 'center', gap: 10,
                                        padding: '6px 10px',
                                        background: 'var(--paper-2)',
                                        border: 'var(--hairline)', borderRadius: 4 }}>
                <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi)',
                              flex: 1 }}>{tool}</span>
                <ToolBar value={d}/>
                <span className="mono" style={{ fontSize: 11,
                              color: d >= 0 ? 'var(--jade)' : 'var(--shu)',
                              minWidth: 48, textAlign: 'right' }}>
                  {d >= 0 ? "+" : ""}{d}%
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* MOE panel */}
      <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderLeft: `2px solid ${vm.color}`,
                     borderRadius: 6, padding: '18px 22px', marginBottom: 22 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
          <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)' }}>議</span>
          <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                          textTransform: 'uppercase' }}>MOE panel reasoning</span>
          <span style={{ flex: 1 }}/>
          <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
            {r.moeReasoning.consensus}
          </span>
        </div>
        <div style={{ fontSize: 14, color: 'var(--sumi)', lineHeight: 1.5,
                       fontWeight: 500, marginBottom: 8 }}>
          {r.moeReasoning.headline}
        </div>
        <p style={{ fontSize: 13, color: 'var(--sumi-2)', lineHeight: 1.65,
                     margin: '0 0 14px' }}>{r.moeReasoning.body}</p>

        {/* Per-model votes */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6,
                       paddingTop: 12, borderTop: 'var(--hairline)' }}>
          {r.moeReasoning.models.map((m, i) => {
            const mv = VERDICT_META[m.verdict];
            return (
              <div key={i} style={{ display: 'grid',
                                     gridTemplateColumns: '120px 14px 1fr',
                                     gap: 10, alignItems: 'flex-start' }}>
                <span className="mono" style={{ fontSize: 11, color: 'var(--sumi)' }}>
                  {m.name}
                </span>
                <span className="kanji" style={{ fontSize: 12, color: mv.color,
                              marginTop: 1 }}>{mv.glyph}</span>
                <span style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.5 }}>
                  {m.note}
                </span>
              </div>
            );
          })}
        </div>

        {r.moeReasoning.suggestedRevision && (
          <div style={{ marginTop: 14, padding: '10px 12px',
                         background: 'var(--paper)', borderRadius: 4,
                         border: 'var(--hairline)' }}>
            <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--shu)',
                           textTransform: 'uppercase', marginBottom: 4 }}>
              Suggested revision
            </div>
            <div style={{ fontSize: 12, color: 'var(--sumi)', lineHeight: 1.55 }}>
              {r.moeReasoning.suggestedRevision}
            </div>
          </div>
        )}
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', gap: 8, alignItems: 'center', paddingTop: 4 }}>
        {r.verdict === "negative" ? (
          <>
            <button style={{ padding: '9px 18px', fontSize: 13,
                              background: 'var(--sumi)', color: 'var(--paper)',
                              border: 'none', borderRadius: 6, cursor: 'pointer',
                              display: 'inline-flex', alignItems: 'center', gap: 8 }}>
              <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>戻</span>
              Revert change
            </button>
            <FlatBtn glyph="改" label="Revise the rule"/>
          </>
        ) : (
          <FlatBtn glyph="改" label="Revise"/>
        )}
        <FlatBtn glyph="観" label="Keep monitoring"/>
        <span style={{ flex: 1 }}/>
        <FlatBtn glyph="納" label="Dismiss" subtle/>
      </div>
    </div>
  );
}

function BeforeAfter({ label, before, after, delta, positive }) {
  return (
    <div style={{ background: 'var(--paper-2)', padding: '14px 16px' }}>
      <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-4)',
                     textTransform: 'uppercase', marginBottom: 8 }}>{label}</div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{before}</span>
        {after && <span style={{ fontSize: 11, color: 'var(--sumi-4)' }}>→</span>}
        {after && (
          <span className="display" style={{ fontSize: 17, fontWeight: 400,
                        color: 'var(--sumi)' }}>{after}</span>
        )}
      </div>
      {delta && (
        <div className="mono" style={{ fontSize: 11,
                       color: positive ? 'var(--jade)' : 'var(--shu)',
                       marginTop: 4 }}>{delta}</div>
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
      <line x1={half} y1={0} x2={half} y2={8} stroke="var(--paper-edge)" strokeWidth="1"/>
      <rect x={positive ? half : half - len} y={2} width={len} height={4}
            fill={positive ? 'var(--jade)' : 'var(--shu)'} rx={1}/>
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
    <div className="sensei" data-screen-label="Observatory · Negative impact alert"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* The alert banner */}
      <div style={{ background: 'var(--shu)',
                     padding: '14px 36px',
                     display: 'flex', alignItems: 'center', gap: 14,
                     color: 'var(--paper)' }}>
        <span className="kanji" style={{ fontSize: 22 }}>警</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                         opacity: 0.8 }}>regression detected</div>
          <div style={{ fontSize: 14, fontWeight: 500 }}>
            A change you accepted on {r.acted} is hurting your FTR.
            Sensei surfaced this for review.
          </div>
        </div>
        <button style={{ fontSize: 11, color: 'var(--paper)',
                          background: 'rgba(255,255,255,0.15)', border: 'none',
                          padding: '6px 12px', borderRadius: 4, cursor: 'pointer' }}>
          dismiss
        </button>
      </div>

      <div style={{ flex: 1, overflow: 'auto', padding: '32px 60px 40px',
                     maxWidth: 920, margin: '0 auto', width: '100%' }}>

        {/* Headline */}
        <div style={{ fontSize: 11, color: 'var(--shu)', letterSpacing: '0.14em',
                       textTransform: 'uppercase', fontWeight: 500, marginBottom: 8 }}>
          Negative impact · {r.window}
        </div>
        <h1 className="display" style={{ fontSize: 32, fontWeight: 300, lineHeight: 1.2,
                                          letterSpacing: '-0.015em',
                                          margin: '0 0 12px', color: 'var(--sumi)' }}>
          {r.title}
        </h1>
        <p style={{ fontSize: 14, color: 'var(--sumi-2)', lineHeight: 1.65, margin: '0 0 28px' }}>
          {r.moeReasoning.headline}
        </p>

        {/* The two big deltas */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr',
                       gap: 14, marginBottom: 28 }}>
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
        <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 6, padding: '20px 24px', marginBottom: 22 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 14 }}>
            <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)' }}>議</span>
            <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                            textTransform: 'uppercase' }}>Why · MOE panel</span>
            <span style={{ flex: 1 }}/>
            <span className="mono" style={{ fontSize: 10.5, color: 'var(--shu)' }}>
              {r.moeReasoning.consensus}
            </span>
          </div>
          <p style={{ fontSize: 13, color: 'var(--sumi)', lineHeight: 1.7,
                       margin: '0 0 16px' }}>{r.moeReasoning.body}</p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8,
                         paddingTop: 14, borderTop: 'var(--hairline)' }}>
            {r.moeReasoning.models.map((m, i) => {
              const mv = VERDICT_META[m.verdict];
              return (
                <div key={i} style={{ display: 'grid',
                                       gridTemplateColumns: '130px 14px 1fr',
                                       gap: 12, alignItems: 'flex-start' }}>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--sumi)' }}>
                    {m.name}
                  </span>
                  <span className="kanji" style={{ fontSize: 12, color: mv.color }}>{mv.glyph}</span>
                  <span style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
                    {m.note}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Suggested revision */}
        {r.moeReasoning.suggestedRevision && (
          <div style={{ background: 'var(--paper)', border: '1px solid var(--shu)',
                         borderRadius: 6, padding: '16px 20px', marginBottom: 28 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)' }}>改</span>
              <span style={{ fontSize: 10.5, letterSpacing: '0.14em', color: 'var(--shu)',
                              textTransform: 'uppercase', fontWeight: 500 }}>
                Recommended fix
              </span>
            </div>
            <div style={{ fontSize: 13, color: 'var(--sumi)', lineHeight: 1.6 }}>
              {r.moeReasoning.suggestedRevision}
            </div>
          </div>
        )}

        {/* Actions */}
        <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
          <button style={{ padding: '10px 20px', fontSize: 13,
                            background: 'var(--sumi)', color: 'var(--paper)',
                            border: 'none', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>戻</span>
            Revert change
          </button>
          <button style={{ padding: '10px 20px', fontSize: 13,
                            background: 'var(--paper-2)', color: 'var(--sumi)',
                            border: 'var(--hairline)', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>改</span>
            Revise the rule
          </button>
          <FlatBtn glyph="観" label="Keep monitoring"/>
          <span style={{ flex: 1 }}/>
          <FlatBtn glyph="納" label="Dismiss" subtle/>
        </div>
      </div>
    </div>
  );
}

function DeltaCard({ label, before, after, delta, dir, bad }) {
  return (
    <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 6, padding: '18px 22px' }}>
      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', marginBottom: 12 }}>{label}</div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
        <span className="display" style={{ fontSize: 28, fontWeight: 300, color: 'var(--sumi-3)',
                      lineHeight: 1 }}>{before}</span>
        <span style={{ fontSize: 14, color: 'var(--sumi-4)' }}>→</span>
        <span className="display" style={{ fontSize: 36, fontWeight: 300,
                      color: bad ? 'var(--shu)' : 'var(--jade)',
                      lineHeight: 1 }}>{after}</span>
        <span style={{ flex: 1 }}/>
        <div style={{ display: 'inline-flex', alignItems: 'center', gap: 4,
                       fontSize: 12, color: bad ? 'var(--shu)' : 'var(--jade)',
                       fontWeight: 500 }}>
          <span style={{ fontSize: 14 }}>{dir === "down" ? "↓" : "↑"}</span>
          <span className="mono">{delta}</span>
        </div>
      </div>
    </div>
  );
}

window.ObsImpact = ObsImpact;
window.ObsNegativeAlert = ObsNegativeAlert;
