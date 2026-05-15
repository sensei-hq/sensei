// Benchmark runner — A/B compare assistant runs with vs without sensei tools.
// Two layouts:
//   A · Dashboard view — corpora list, runs table, single-run detail card
//   B · Lab notebook view — single run as a long-scroll narrative report

const { useState: bnS } = React;

// ─── Shared bits ───────────────────────────────────────────
function BnHero({ subtitle, title, blurb, stats }) {
  return (
    <div style={{
 borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
      <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>較</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-1" >{subtitle}</div>
        <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                          color: 'var(--ink)'
}}>{title}</h1>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     maxWidth: 760, lineHeight: 1.55
}} className="mt-1 mb-0" >{blurb}</p>
      </div>
      <div style={{
 borderLeft: 'var(--hairline)',
                     display: 'flex'
}} className="gap-5 pl-5" >
        {stats.map((s, i) => (
          <div key={i} style={{ textAlign: 'right' }}>
            <div className={s.mono ? "mono" : "display"} style={{
              fontSize: s.mono ? 13 : 22,
              color: s.accent ? 'var(--accent)' : 'var(--ink)',
              fontWeight: 400, lineHeight: 1
            }}>{s.n}</div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                           textTransform: 'uppercase'
}} className="mt-1" >{s.l}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Layout A: Dashboard ───────────────────────────────────
function BenchmarkRunnerDashboard() {
  const B = window.EXT_DATA.benchmark;
  const [activeRun, setActiveRun] = bnS(B.runs[0].id);
  const [activeCorpus, setActiveCorpus] = bnS(B.corpora[0].id);

  const run = B.runs.find(r => r.id === activeRun);
  const corpus = B.corpora.find(c => c.id === activeCorpus);

  return (
    <div className="sensei" data-screen-label="Benchmark · Dashboard"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <BnHero
        subtitle="Configure · Benchmark"
        title="Sensei vs no-sensei. Same model, same tasks."
        blurb="A corpus is a repo with a /tasks folder. Sensei runs each task twice: once without its tools, once with. The diff is the value."
        stats={[
          { n: B.corpora.length, l: "corpora" },
          { n: B.runs.length, l: "runs", mono: true },
          { n: `+${Math.round(B.runs.reduce((s,r)=>s+(r.b.score-r.a.score),0)/B.runs.length*100)}%`,
            l: "avg score lift", mono: true, accent: true },
        ]}
      />

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1fr 1.4fr' }}>
        {/* Left: corpora + runs lists */}
        <div style={{
 overflow: 'auto',
                       borderRight: 'var(--hairline)'
}} className="py-5 px-5" >
          <BnSection title="Corpora">
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
              {B.corpora.map(c => (
                <button key={c.id} onClick={() => setActiveCorpus(c.id)} style={{
                  textAlign: 'left', borderRadius: 5,
                  background: activeCorpus === c.id ? 'var(--paper-3)' : 'var(--paper-2)',
                  border: activeCorpus === c.id ? '1px solid var(--ink-3)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-3 px-3" >
                  <div style={{
 display: 'flex', alignItems: 'center'
}} className="gap-2 mb-1" >
                    <span style={{ fontSize: 13, color: 'var(--ink)' }}>{c.label}</span>
                    <span style={{ fontSize: 11,
                      color: c.kind === "private" ? 'var(--warning)' : 'var(--success)',
                      letterSpacing: '0.12em', textTransform: 'uppercase' }}>{c.kind}</span>
                  </div>
                  <div className="mono mb-1" style={{
 fontSize: 11, color: 'var(--ink-3)'
}}>{c.repo}</div>
                  <div style={{
 display: 'flex', fontSize: 11,
                                 color: 'var(--ink-4)'
}} className="gap-2" >
                    <span>{c.tasks} tasks</span>
                    <span>· {c.langs.join(', ')}</span>
                    <span>· {c.lastSync}</span>
                  </div>
                </button>
              ))}
              <button style={{
 fontSize: 11, color: 'var(--ink-3)',
                background: 'transparent', border: '1px dashed var(--edge)',
                borderRadius: 4, cursor: 'pointer'
}} className="p-2" >
                + import corpus from repo
              </button>
            </div>
          </BnSection>

          <BnSection title="Recent runs">
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
              {B.runs.map(r => {
                const win = r.delta.passed > 0;
                return (
                  <button key={r.id} onClick={() => setActiveRun(r.id)} style={{
                    textAlign: 'left', borderRadius: 4,
                    background: activeRun === r.id ? 'var(--paper-2)' : 'transparent',
                    border: activeRun === r.id ? '1px solid var(--ink-3)' : 'var(--hairline)',
                    cursor: 'pointer', fontFamily: 'var(--font-ui)',
                    display: 'grid', gridTemplateColumns: '1fr auto auto', alignItems: 'center'
}} className="py-2 px-3 gap-2" >
                    <div>
                      <div style={{ fontSize: 13, color: 'var(--ink)' }}>
                        {r.corpus}  <span className="mono" style={{ color: 'var(--ink-4)' }}>· {r.id}</span>
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
                        {r.started}  ·  {r.duration}
                      </div>
                    </div>
                    <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                      {r.b.passed}/{r.b.total}
                    </span>
                    <span style={{ fontSize: 11,
                      color: win ? 'var(--success)' : 'var(--warning)',
                      fontFamily: 'var(--font-mono)' }}>
                      {win ? '+' : ''}{r.delta.passed}
                    </span>
                  </button>
                );
              })}
            </div>
          </BnSection>

          {/* Run new */}
          <div style={{
 background: 'var(--paper-2)',
                         border: 'var(--hairline)', borderRadius: 6
}} className="py-3 px-4" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                           textTransform: 'uppercase'
}} className="mb-2" >New run</div>
            <p style={{
 fontSize: 11, color: 'var(--ink-3)',
                         lineHeight: 1.55
}} className="mt-0 mb-2" >
              Will execute every task on <strong style={{ color: 'var(--ink-2)', fontWeight: 500 }}>
              {corpus.label}</strong> twice — first without sensei, then with sensei + MCPs enabled.
            </p>
            <button style={{
 fontSize: 13, background: 'var(--ink)',
              color: 'var(--paper)', borderRadius: 5, border: 'none',
              cursor: 'pointer', width: '100%', fontFamily: 'var(--font-ui)'
}} className="py-2 px-3" >Run benchmark  ({corpus.tasks} tasks · ~{Math.ceil(corpus.tasks*2.5)}m)</button>
          </div>
        </div>

        {/* Right: run detail */}
        <div style={{ overflow: 'auto' }} className="py-5 px-6" >
          {/* Run header */}
          <div className="mb-4" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-1" >
              Run {run.id}  ·  {run.corpus}  ·  {run.started}
            </div>
            <h2 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                              color: 'var(--ink)'
}}>{run.verdict}</h2>
          </div>

          {/* A vs B card */}
          <div style={{
 display: 'grid', gridTemplateColumns: '1fr 24px 1fr'
}} className="gap-3 mb-5" >
            <BnRunCard side="A" data={run.a} accent="var(--ink-3)"/>
            <div style={{ display: 'flex', flexDirection: 'column',
                           alignItems: 'center', justifyContent: 'center',
                           color: 'var(--ink-4)' }}>
              <span className="display" style={{ fontSize: 28, lineHeight: 1 }}>vs</span>
            </div>
            <BnRunCard side="B" data={run.b} accent="var(--accent)" highlight/>
          </div>

          {/* Delta strip */}
          <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)'
}} className="gap-3 mb-5" >
            <BnDelta label="passed"  v={run.delta.passed}  unit="" good={run.delta.passed > 0}/>
            <BnDelta label="score"   v={run.delta.score}   unit="" pct good={run.delta.score > 0}/>
            <BnDelta label="tool calls" v={run.delta.toolCalls} unit="" good={run.delta.toolCalls < 0} invert/>
            <BnDelta label="tokens"  v={run.delta.tokens}   unit="" good={run.delta.tokens < 0} invert k/>
          </div>

          {/* Task table */}
          <div className="mb-3" >
            <h3 className="display mt-0 mb-1" style={{
 fontSize: 15, fontWeight: 400, color: 'var(--ink)'
}}>
              Per-task results
            </h3>
            <p style={{ fontSize: 11, color: 'var(--ink-3)' }} className="m-0" >
              {B.taskBreakdown.length} of {run.b.total} tasks shown.
            </p>
          </div>
          <div style={{ border: 'var(--hairline)', borderRadius: 6, overflow: 'hidden' }}>
            <div style={{
 display: 'grid', gridTemplateColumns: '60px 1fr 70px 70px 1.4fr', background: 'var(--paper-2)',
                           borderBottom: 'var(--hairline)',
                           fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                           textTransform: 'uppercase'
}} className="py-2 px-3" >
              <span>id</span><span>task</span>
              <span style={{ textAlign: 'center' }}>without</span>
              <span style={{ textAlign: 'center' }}>with</span>
              <span>note</span>
            </div>
            {B.taskBreakdown.map((t, i) => (
              <div key={t.id} style={{
                display: 'grid', gridTemplateColumns: '60px 1fr 70px 70px 1.4fr',
                borderBottom: i < B.taskBreakdown.length-1 ? 'var(--hairline)' : 'none',
                fontSize: 13, alignItems: 'center'
}} className="py-2 px-3" >
                <span className="mono" style={{ color: 'var(--ink-3)' }}>{t.id}</span>
                <span style={{ color: 'var(--ink)' }}>{t.title}</span>
                <BnPF v={t.a}/>
                <BnPF v={t.b}/>
                <span style={{ color: 'var(--ink-3)', fontSize: 11 }}>{t.note}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function BnRunCard({ side, data, accent, highlight }) {
  return (
    <div style={{
 borderRadius: 6,
      background: highlight ? 'var(--paper-2)' : 'transparent',
      border: highlight ? '1px solid var(--accent)' : 'var(--hairline)'
}} className="py-4 px-4" >
      <div style={{
 display: 'flex', alignItems: 'baseline',
                     justifyContent: 'space-between'
}} className="mb-3" >
        <span className="display" style={{ fontSize: 28, color: accent, lineHeight: 1 }}>{side}</span>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: accent,
                        textTransform: 'uppercase' }}>
          {side === "A" ? "without sensei" : "with sensei"}
        </span>
      </div>
      <div className="mono mb-3" style={{
 fontSize: 11, color: 'var(--ink-2)'
}}>{data.label}</div>
      <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr',
                     gap: '4px 12px', fontSize: 11 }}>
        <span style={{ color: 'var(--ink-4)' }}>passed</span>
        <span className="mono" style={{ color: 'var(--ink)' }}>
          {data.passed} / {data.total}
        </span>
        <span style={{ color: 'var(--ink-4)' }}>score</span>
        <span className="mono" style={{ color: 'var(--ink)' }}>
          {(data.score * 100).toFixed(0)}%
        </span>
        <span style={{ color: 'var(--ink-4)' }}>tool calls</span>
        <span className="mono" style={{ color: 'var(--ink-2)' }}>{data.toolCalls}</span>
        <span style={{ color: 'var(--ink-4)' }}>tokens</span>
        <span className="mono" style={{ color: 'var(--ink-2)' }}>
          {(data.tokens/1000).toFixed(0)}k
        </span>
      </div>
    </div>
  );
}

function BnDelta({ label, v, good, pct, k, invert }) {
  const color = v === 0 ? 'var(--ink-3)' :
                good ? 'var(--success)' : 'var(--warning)';
  const sign = v > 0 ? '+' : '';
  let display;
  if (pct) display = `${sign}${(v*100).toFixed(0)}%`;
  else if (k) display = `${sign}${(v/1000).toFixed(0)}k`;
  else display = `${sign}${v}`;
  return (
    <div style={{
 background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 5
}} className="py-3 px-3" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase'
}} className="mb-1" >{label}</div>
      <div className="mono" style={{ fontSize: 17, color, lineHeight: 1 }}>{display}</div>
    </div>
  );
}

function BnPF({ v }) {
  return (
    <span style={{ textAlign: 'center',
      color: v === "pass" ? 'var(--success)' : 'var(--warning)',
      fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
      fontFamily: 'var(--font-mono)' }}>{v}</span>
  );
}

function BnSection({ title, children }) {
  return (
    <section className="mb-5" >
      <h3 className="display mt-0 mb-2" style={{
 fontSize: 15, fontWeight: 400,
                                        color: 'var(--ink)'
}}>{title}</h3>
      {children}
    </section>
  );
}

// ─── Layout B: Lab notebook ────────────────────────────────
function BenchmarkRunnerNotebook() {
  const B = window.EXT_DATA.benchmark;
  const run = B.runs[0];
  const corpus = B.corpora.find(c => c.id === run.corpus);

  return (
    <div className="sensei" data-screen-label="Benchmark · Notebook"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <BnHero
        subtitle="Benchmark · run report"
        title={`${corpus.label} · ${run.id}`}
        blurb={`Two runs of the same corpus. ${run.a.label} vs ${run.b.label}.`}
        stats={[
          { n: run.duration, l: "duration", mono: true },
          { n: run.b.total, l: "tasks", mono: true },
          { n: `+${(run.delta.score*100).toFixed(0)}%`, l: "score lift", mono: true, accent: true },
        ]}
      />

      <div style={{
 flex: 1, minHeight: 0, overflow: 'auto',
                     maxWidth: 1100, width: '100%'
}} className="pt-6 pb-8 px-8 mx-auto" >

        {/* Abstract */}
        <NbBlock label="Abstract">
          <p style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.7, maxWidth: 760
}} className="m-0" >
            We executed the <strong>{corpus.label}</strong> corpus ({corpus.tasks} tasks)
            twice with <span className="mono" style={{ color: 'var(--accent)' }}>{run.a.label.split('·')[0].trim()}</span>:
            once with sensei's tools, MCPs and memory disabled (run A), and once with them
            fully active (run B). <strong>{run.verdict}</strong>
          </p>
        </NbBlock>

        {/* Setup */}
        <NbBlock label="Setup · what changed between A and B">
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-3" >
            <div style={{
 borderRadius: 5,
                           background: 'var(--paper-2)', border: 'var(--hairline)'
}} className="py-3 px-4" >
              <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-2" >
                <span className="display" style={{ fontSize: 22, color: 'var(--ink-3)' }}>A</span>
                <span style={{ fontSize: 11, letterSpacing: '0.14em',
                                color: 'var(--ink-3)', textTransform: 'uppercase' }}>
                  baseline
                </span>
              </div>
              <ul style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.7
}} className="m-0 pl-4" >
                <li>Bare assistant. No sensei extensions, no MCPs.</li>
                <li>No project memory loaded.</li>
                <li>Single fallback model.</li>
                <li>Tool budget: assistant defaults.</li>
              </ul>
            </div>
            <div style={{
 borderRadius: 5,
                           background: 'var(--paper-2)', border: '1px solid var(--accent)'
}} className="py-3 px-4" >
              <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-2" >
                <span className="display" style={{ fontSize: 22, color: 'var(--accent)' }}>B</span>
                <span style={{ fontSize: 11, letterSpacing: '0.14em',
                                color: 'var(--accent)', textTransform: 'uppercase' }}>
                  with sensei
                </span>
              </div>
              <ul style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.7
}} className="m-0 pl-4" >
                <li>All installed extensions enabled.</li>
                <li>Project memories surfaced on context match.</li>
                <li>Fallback chain · MOE on for high-stakes calls.</li>
                <li>MCPs: tsserver, fs-read, react-devtools, session-replay.</li>
              </ul>
            </div>
          </div>
        </NbBlock>

        {/* Headline numbers */}
        <NbBlock label="Headline numbers">
          <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)'
}} className="gap-3 mb-3" >
            <BnDelta label="passed"     v={run.delta.passed} good={run.delta.passed > 0}/>
            <BnDelta label="score"      v={run.delta.score} pct good={run.delta.score > 0}/>
            <BnDelta label="tool calls" v={run.delta.toolCalls} good={run.delta.toolCalls < 0} invert/>
            <BnDelta label="tokens"     v={run.delta.tokens} k good={run.delta.tokens < 0} invert/>
          </div>
          <p style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.6 }} className="m-0" >
            Sensei produced more passes with fewer tool calls and fewer tokens —
            efficiency improved on every axis we measure.
          </p>
        </NbBlock>

        {/* Per-task narrative table */}
        <NbBlock label="Per-task results">
          <div style={{ border: 'var(--hairline)', borderRadius: 6, overflow: 'hidden' }}>
            <div style={{
 display: 'grid', gridTemplateColumns: '60px 1fr 60px 60px 1.6fr', background: 'var(--paper-2)',
                           borderBottom: 'var(--hairline)',
                           fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                           textTransform: 'uppercase'
}} className="py-2 px-3" >
              <span>id</span><span>task</span>
              <span style={{ textAlign: 'center' }}>A</span>
              <span style={{ textAlign: 'center' }}>B</span>
              <span>commentary</span>
            </div>
            {B.taskBreakdown.map((t, i) => (
              <div key={t.id} style={{
                display: 'grid', gridTemplateColumns: '60px 1fr 60px 60px 1.6fr',
                borderBottom: i < B.taskBreakdown.length-1 ? 'var(--hairline)' : 'none',
                fontSize: 13, alignItems: 'center'
}} className="py-2 px-3" >
                <span className="mono" style={{ color: 'var(--ink-3)' }}>{t.id}</span>
                <span style={{ color: 'var(--ink)' }}>{t.title}</span>
                <BnPF v={t.a}/>
                <BnPF v={t.b}/>
                <span style={{ color: 'var(--ink-3)', fontSize: 11 }}>{t.note}</span>
              </div>
            ))}
          </div>
        </NbBlock>

        {/* Where sensei made the difference */}
        <NbBlock label="Where sensei made the difference">
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-3" >
            <div style={{
 background: 'var(--paper-2)',
                           border: 'var(--hairline)', borderRadius: 5
}} className="py-3 px-4" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--accent)',
                             textTransform: 'uppercase'
}} className="mb-2" >
                Tasks won by skill / agent triggers
              </div>
              <ul style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.7
}} className="m-0 pl-4" >
                <li><strong>t01</strong> — react-perf-watch caught render-thrash.</li>
                <li><strong>t02</strong> — boundary memory surfaced before the touch.</li>
                <li><strong>t04</strong> — migration-runner agent generated the SQL.</li>
                <li><strong>t08</strong> — doc-drift skill caught README mismatch.</li>
              </ul>
            </div>
            <div style={{
 background: 'var(--paper-2)',
                           border: 'var(--hairline)', borderRadius: 5
}} className="py-3 px-4" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--warning)',
                             textTransform: 'uppercase'
}} className="mb-2" >
                Where both still failed
              </div>
              <ul style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.7
}} className="m-0 pl-4" >
                <li><strong>t06</strong> — borrow-check fix in canvas/event.rs.
                    No memory exists yet for this pattern. Tagged as a candidate.</li>
              </ul>
            </div>
          </div>
        </NbBlock>

        {/* Reproduce */}
        <NbBlock label="Reproduce">
          <pre className="mono py-3 px-3 m-0" style={{
 fontSize: 13, color: 'var(--ink-2)',
            background: 'var(--paper-2)', border: 'var(--hairline)',
            borderRadius: 5,
            lineHeight: 1.7, whiteSpace: 'pre-wrap'
}}>
{`$ sensei bench run --corpus ${corpus.repo} \\
    --model claude-sonnet-4.5 \\
    --baseline none \\
    --variant default

# resume:
$ sensei bench resume ${run.id}`}
          </pre>
        </NbBlock>

        <div style={{ display: 'flex' }} className="gap-2 mt-2" >
          <button style={{
 fontSize: 13, background: 'var(--ink)',
            color: 'var(--paper)', borderRadius: 5, border: 'none',
            cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-2 px-4" >Re-run on latest sensei</button>
          <button style={{
 fontSize: 13, background: 'transparent',
            color: 'var(--ink-2)', borderRadius: 5,
            border: '1px solid var(--ink-3)', cursor: 'pointer',
            fontFamily: 'var(--font-ui)'
}} className="py-2 px-4" >Export markdown</button>
          <button style={{
 fontSize: 13, background: 'transparent',
            color: 'var(--ink-3)', borderRadius: 5, border: 'none',
            cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-2 px-3" >Share with collective →</button>
        </div>
      </div>
    </div>
  );
}

function NbBlock({ label, children }) {
  return (
    <section className="mb-6" >
      <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', borderBottom: 'var(--hairline)'
}} className="mb-3 pb-2" >
        {label}
      </div>
      {children}
    </section>
  );
}

window.BenchmarkRunnerDashboard = BenchmarkRunnerDashboard;
window.BenchmarkRunnerNotebook = BenchmarkRunnerNotebook;
