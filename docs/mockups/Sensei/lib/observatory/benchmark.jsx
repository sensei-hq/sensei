// Benchmark runner — A/B compare assistant runs with vs without sensei tools.
// Two layouts:
//   A · Dashboard view — corpora list, runs table, single-run detail card
//   B · Lab notebook view — single run as a long-scroll narrative report

const { useState: bnS } = React;

// ─── Shared bits ───────────────────────────────────────────
function BnHero({ subtitle, title, blurb, stats }) {
  return (
    <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
      <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>較</div>
      <div className="flex-1 min-w-0" >
        <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >{subtitle}</div>
        <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>{title}</h1>
        <p style={{
 fontSize: 13,
 maxWidth: 760, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >{blurb}</p>
      </div>
      <div className="gap-6 pl-6 border-l flex" >
        {stats.map((s, i) => (
          <div className="text-right" key={i} >
            <div className={s.mono ? "mono" : "display"} style={{
              fontSize: s.mono ? 13 : 22,
              color: s.accent ? 'var(--accent)' : 'var(--ink)',
              fontWeight: 400, lineHeight: 1
            }}>{s.n}</div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 text-ink-4 uppercase" >{s.l}</div>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Benchmark · Dashboard"
 >
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

      <div className="flex-1 min-h-0 grid" style={{
 gridTemplateColumns: '1fr 1.4fr' }}>
        {/* Left: corpora + runs lists */}
        <div className="py-6 px-6 overflow-auto border-r" >
          <BnSection title="Corpora">
            <div className="gap-1 flex flex-col" >
              {B.corpora.map(c => (
                <button key={c.id} onClick={() => setActiveCorpus(c.id)} style={{ borderRadius: 5,
 background: activeCorpus === c.id ? 'var(--paper-3)' : 'var(--paper-2)',
 border: activeCorpus === c.id ? '1px solid var(--ink-3)' : 'var(--hairline)', fontFamily: 'var(--font-ui)'
 }} className="py-3 px-3 text-left cursor-pointer" >
                  <div className="gap-2 mb-1 flex items-center" >
                    <span className="text-ink" style={{ fontSize: 13 }}>{c.label}</span>
                    <span className="uppercase" style={{ fontSize: 11,
 color: c.kind === "private" ? 'var(--warning)' : 'var(--success)',
 letterSpacing: '0.12em' }}>{c.kind}</span>
                  </div>
                  <div className="mono mb-1 text-ink-3" style={{
 fontSize: 11 }}>{c.repo}</div>
                  <div style={{ fontSize: 11 }} className="gap-2 flex text-ink-4" >
                    <span>{c.tasks} tasks</span>
                    <span>· {c.langs.join(', ')}</span>
                    <span>· {c.lastSync}</span>
                  </div>
                </button>
              ))}
              <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 text-ink-3 bg-transparent cursor-pointer" >
                + import corpus from repo
              </button>
            </div>
          </BnSection>

          <BnSection title="Recent runs">
            <div className="gap-1 flex flex-col" >
              {B.runs.map(r => {
                const win = r.delta.passed > 0;
                return (
                  <button key={r.id} onClick={() => setActiveRun(r.id)} style={{ borderRadius: 4,
 background: activeRun === r.id ? 'var(--paper-2)' : 'transparent',
 border: activeRun === r.id ? '1px solid var(--ink-3)' : 'var(--hairline)', fontFamily: 'var(--font-ui)', gridTemplateColumns: '1fr auto auto' }} className="py-2 px-3 gap-2 text-left cursor-pointer grid items-center" >
                    <div>
                      <div className="text-ink" style={{ fontSize: 13 }}>
                        {r.corpus}  <span className="mono text-ink-4" >· {r.id}</span>
                      </div>
                      <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
                        {r.started}  ·  {r.duration}
                      </div>
                    </div>
                    <span className="mono text-ink-3" style={{ fontSize: 11 }}>
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
          <div style={{ borderRadius: 6
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >New run</div>
            <p style={{
 fontSize: 11,
 lineHeight: 1.55
 }} className="mt-0 mb-2 text-ink-3" >
              Will execute every task on <strong className="text-ink-2 font-medium" >
              {corpus.label}</strong> twice — first without sensei, then with sensei + MCPs enabled.
            </p>
            <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer w-full" >Run benchmark  ({corpus.tasks} tasks · ~{Math.ceil(corpus.tasks*2.5)}m)</button>
          </div>
        </div>

        {/* Right: run detail */}
        <div className="py-6 px-8 overflow-auto" >
          {/* Run header */}
          <div className="mb-4" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
              Run {run.id}  ·  {run.corpus}  ·  {run.started}
            </div>
            <h2 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>{run.verdict}</h2>
          </div>

          {/* A vs B card */}
          <div style={{ gridTemplateColumns: '1fr 24px 1fr'
 }} className="gap-3 mb-6 grid" >
            <BnRunCard side="A" data={run.a} accent="var(--ink-3)"/>
            <div className="flex flex-col items-center justify-center text-ink-4" >
              <span className="display" style={{ fontSize: 28, lineHeight: 1 }}>vs</span>
            </div>
            <BnRunCard side="B" data={run.b} accent="var(--accent)" highlight/>
          </div>

          {/* Delta strip */}
          <div style={{ gridTemplateColumns: 'repeat(4, 1fr)'
 }} className="gap-3 mb-6 grid" >
            <BnDelta label="passed"  v={run.delta.passed}  unit="" good={run.delta.passed > 0}/>
            <BnDelta label="score"   v={run.delta.score}   unit="" pct good={run.delta.score > 0}/>
            <BnDelta label="tool calls" v={run.delta.toolCalls} unit="" good={run.delta.toolCalls < 0} invert/>
            <BnDelta label="tokens"  v={run.delta.tokens}   unit="" good={run.delta.tokens < 0} invert k/>
          </div>

          {/* Task table */}
          <div className="mb-3" >
            <h3 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 15 }}>
              Per-task results
            </h3>
            <p style={{ fontSize: 11 }} className="m-0 text-ink-3" >
              {B.taskBreakdown.length} of {run.b.total} tasks shown.
            </p>
          </div>
          <div className="border border-paper-edge overflow-hidden" style={{ borderRadius: 6 }}>
            <div style={{ gridTemplateColumns: '60px 1fr 70px 70px 1.4fr',
 fontSize: 11, letterSpacing: '0.14em' }} className="py-2 px-3 grid bg-paper-2 border-b text-ink-4 uppercase" >
              <span>id</span><span>task</span>
              <span className="text-center" >without</span>
              <span className="text-center" >with</span>
              <span>note</span>
            </div>
            {B.taskBreakdown.map((t, i) => (
              <div key={t.id} style={{ gridTemplateColumns: '60px 1fr 70px 70px 1.4fr',
 borderBottom: i < B.taskBreakdown.length-1 ? 'var(--hairline)' : 'none',
 fontSize: 13 }} className="py-2 px-3 grid items-center" >
                <span className="mono text-ink-3" >{t.id}</span>
                <span className="text-ink" >{t.title}</span>
                <BnPF v={t.a}/>
                <BnPF v={t.b}/>
                <span className="text-ink-3" style={{ fontSize: 11 }}>{t.note}</span>
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
      <div className="mb-3 flex items-baseline justify-between" >
        <span className="display" style={{ fontSize: 28, color: accent, lineHeight: 1 }}>{side}</span>
        <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.14em', color: accent }}>
          {side === "A" ? "without sensei" : "with sensei"}
        </span>
      </div>
      <div className="mono mb-3 text-ink-2" style={{
 fontSize: 11 }}>{data.label}</div>
      <div className="grid" style={{ gridTemplateColumns: 'auto 1fr',
 gap: '4px 12px', fontSize: 11 }}>
        <span className="text-ink-4" >passed</span>
        <span className="mono text-ink" >
          {data.passed} / {data.total}
        </span>
        <span className="text-ink-4" >score</span>
        <span className="mono text-ink" >
          {(data.score * 100).toFixed(0)}%
        </span>
        <span className="text-ink-4" >tool calls</span>
        <span className="mono text-ink-2" >{data.toolCalls}</span>
        <span className="text-ink-4" >tokens</span>
        <span className="mono text-ink-2" >
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
    <div style={{ borderRadius: 5
 }} className="py-3 px-3 bg-paper-2 border border-paper-edge" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-4 uppercase" >{label}</div>
      <div className="mono" style={{ fontSize: 17, color, lineHeight: 1 }}>{display}</div>
    </div>
  );
}

function BnPF({ v }) {
  return (
    <span className="text-center uppercase" style={{
 color: v === "pass" ? 'var(--success)' : 'var(--warning)',
 fontSize: 11, letterSpacing: '0.12em',
 fontFamily: 'var(--font-mono)' }}>{v}</span>
  );
}

function BnSection({ title, children }) {
  return (
    <section className="mb-6" >
      <h3 className="display mt-0 mb-2 font-normal text-ink" style={{
 fontSize: 15 }}>{title}</h3>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Benchmark · Notebook"
 >
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
 maxWidth: 1100 }} className="pt-8 pb-16 px-16 mx-auto flex-1 min-h-0 overflow-auto w-full" >

        {/* Abstract */}
        <NbBlock label="Abstract">
          <p style={{
 fontSize: 13, lineHeight: 1.7, maxWidth: 760
 }} className="m-0 text-ink-2" >
            We executed the <strong>{corpus.label}</strong> corpus ({corpus.tasks} tasks)
            twice with <span className="mono text-accent" >{run.a.label.split('·')[0].trim()}</span>:
            once with sensei's tools, MCPs and memory disabled (run A), and once with them
            fully active (run B). <strong>{run.verdict}</strong>
          </p>
        </NbBlock>

        {/* Setup */}
        <NbBlock label="Setup · what changed between A and B">
          <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-3 grid" >
            <div style={{
 borderRadius: 5 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
              <div className="gap-2 mb-2 flex items-baseline" >
                <span className="display text-ink-3" style={{ fontSize: 22 }}>A</span>
                <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
                  baseline
                </span>
              </div>
              <ul style={{
 fontSize: 13, lineHeight: 1.7
 }} className="m-0 pl-4 text-ink-2" >
                <li>Bare assistant. No sensei extensions, no MCPs.</li>
                <li>No project memory loaded.</li>
                <li>Single fallback model.</li>
                <li>Tool budget: assistant defaults.</li>
              </ul>
            </div>
            <div style={{
 borderRadius: 5, border: '1px solid var(--accent)'
 }} className="py-3 px-4 bg-paper-2" >
              <div className="gap-2 mb-2 flex items-baseline" >
                <span className="display text-accent" style={{ fontSize: 22 }}>B</span>
                <span className="text-accent uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
                  with sensei
                </span>
              </div>
              <ul style={{
 fontSize: 13, lineHeight: 1.7
 }} className="m-0 pl-4 text-ink-2" >
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
          <div style={{ gridTemplateColumns: 'repeat(4, 1fr)'
 }} className="gap-3 mb-3 grid" >
            <BnDelta label="passed"     v={run.delta.passed} good={run.delta.passed > 0}/>
            <BnDelta label="score"      v={run.delta.score} pct good={run.delta.score > 0}/>
            <BnDelta label="tool calls" v={run.delta.toolCalls} good={run.delta.toolCalls < 0} invert/>
            <BnDelta label="tokens"     v={run.delta.tokens} k good={run.delta.tokens < 0} invert/>
          </div>
          <p style={{ fontSize: 13, lineHeight: 1.6 }} className="m-0 text-ink-3" >
            Sensei produced more passes with fewer tool calls and fewer tokens —
            efficiency improved on every axis we measure.
          </p>
        </NbBlock>

        {/* Per-task narrative table */}
        <NbBlock label="Per-task results">
          <div className="border border-paper-edge overflow-hidden" style={{ borderRadius: 6 }}>
            <div style={{ gridTemplateColumns: '60px 1fr 60px 60px 1.6fr',
 fontSize: 11, letterSpacing: '0.14em' }} className="py-2 px-3 grid bg-paper-2 border-b text-ink-4 uppercase" >
              <span>id</span><span>task</span>
              <span className="text-center" >A</span>
              <span className="text-center" >B</span>
              <span>commentary</span>
            </div>
            {B.taskBreakdown.map((t, i) => (
              <div key={t.id} style={{ gridTemplateColumns: '60px 1fr 60px 60px 1.6fr',
 borderBottom: i < B.taskBreakdown.length-1 ? 'var(--hairline)' : 'none',
 fontSize: 13 }} className="py-2 px-3 grid items-center" >
                <span className="mono text-ink-3" >{t.id}</span>
                <span className="text-ink" >{t.title}</span>
                <BnPF v={t.a}/>
                <BnPF v={t.b}/>
                <span className="text-ink-3" style={{ fontSize: 11 }}>{t.note}</span>
              </div>
            ))}
          </div>
        </NbBlock>

        {/* Where sensei made the difference */}
        <NbBlock label="Where sensei made the difference">
          <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-3 grid" >
            <div style={{ borderRadius: 5
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-accent uppercase" >
                Tasks won by skill / agent triggers
              </div>
              <ul style={{
 fontSize: 13, lineHeight: 1.7
 }} className="m-0 pl-4 text-ink-2" >
                <li><strong>t01</strong> — react-perf-watch caught render-thrash.</li>
                <li><strong>t02</strong> — boundary memory surfaced before the touch.</li>
                <li><strong>t04</strong> — migration-runner agent generated the SQL.</li>
                <li><strong>t08</strong> — doc-drift skill caught README mismatch.</li>
              </ul>
            </div>
            <div style={{ borderRadius: 5
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-warning uppercase" >
                Where both still failed
              </div>
              <ul style={{
 fontSize: 13, lineHeight: 1.7
 }} className="m-0 pl-4 text-ink-2" >
                <li><strong>t06</strong> — borrow-check fix in canvas/event.rs.
                    No memory exists yet for this pattern. Tagged as a candidate.</li>
              </ul>
            </div>
          </div>
        </NbBlock>

        {/* Reproduce */}
        <NbBlock label="Reproduce">
          <pre className="mono py-3 px-3 m-0 text-ink-2 bg-paper-2 border border-paper-edge" style={{
 fontSize: 13,
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

        <div className="gap-2 mt-2 flex" >
          <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-4 bg-ink text-paper border-0 cursor-pointer" >Re-run on latest sensei</button>
          <button style={{
 fontSize: 13, borderRadius: 5,
 border: '1px solid var(--ink-3)',
 fontFamily: 'var(--font-ui)'
 }} className="py-2 px-4 bg-transparent text-ink-2 cursor-pointer" >Export markdown</button>
          <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-3 bg-transparent text-ink-3 border-0 cursor-pointer" >Share with collective →</button>
        </div>
      </div>
    </div>
  );
}

function NbBlock({ label, children }) {
  return (
    <section className="mb-8" >
      <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-3 pb-2 text-ink-4 uppercase border-b" >
        {label}
      </div>
      {children}
    </section>
  );
}

window.BenchmarkRunnerDashboard = BenchmarkRunnerDashboard;
window.BenchmarkRunnerNotebook = BenchmarkRunnerNotebook;
