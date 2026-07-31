// Document traceability — drift visibility before it causes harm.
//
// Per-project rollup → drill into a doc → each reference with status.
// Drifted/broken references surface a "fix drift" prompt that ships off
// to the assistant.

const { useState: dtS } = React;

function ObsTraceability({ state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="巫"
    emptyTitle="Nothing to trace yet"
    emptyHint="Once sensei indexes a project's docs and links them to code, drift and broken references surface here."
    errorHint="Couldn't load traceability. Try again." onRetry={() => {}} />;
  const T = window.UPGRADES.trace;
  const [project, setProject] = dtS(T.projectRollup[0].id);
  const [openDocId, setOpenDocId] = dtS(T.docs.find(d => d.project === T.projectRollup[0].id)?.id);

  const projectDocs = T.docs.filter(d => d.project === project);
  const doc = T.docs.find(d => d.id === openDocId) || projectDocs[0];

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Traceability"
 >

      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>巻</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Observatory · Document traceability
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            Where the docs and the code disagree.
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            Every doc-to-symbol link, checked nightly. Drift becomes visible
            here before someone reads stale docs and writes the wrong thing.
          </p>
        </div>
      </div>

      {/* Project rollup strip */}
      <div className="py-3 px-8 gap-0 border-b flex" >
        {T.projectRollup.map(p => {
          const on = project === p.id;
          const tot = p.current + p.drifted + p.broken;
          return (
            <button key={p.id}
 onClick={() => { setProject(p.id);
 const fd = T.docs.find(d => d.project === p.id);
 setOpenDocId(fd?.id); }}
 style={{
 background: on ? 'var(--paper-2)' : 'transparent', borderRadius: 6 }} className="py-2 px-4 mr-2 flex-1 text-left border border-paper-edge cursor-pointer" >
              <div className="gap-2 mb-1 flex items-baseline" >
                <span className="kanji" style={{ fontSize: 13,
                              color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{p.kanji}</span>
                <span className="mono" style={{ fontSize: 11,
                              color: on ? 'var(--ink)' : 'var(--ink-2)' }}>{p.name}</span>
                <span className="flex-1" />
                <span className="mono" style={{ fontSize: 11,
                              color: p.healthPct >= 0.9 ? 'var(--success)' :
                                     p.healthPct >= 0.8 ? 'var(--ink-2)' : 'var(--warning)' }}>
                  {Math.round(p.healthPct*100)}%
                </span>
              </div>
              <HealthBar current={p.current} drifted={p.drifted} broken={p.broken}/>
              <div style={{
 fontSize: 11 }} className="gap-3 mt-1 flex text-ink-3" >
                <span>{p.docs} docs</span>
                <span>{p.links} links</span>
                <span className="text-warning" >{p.drifted} drifted</span>
                <span className="text-accent" >{p.broken} broken</span>
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex-1 grid min-h-0" style={{
 gridTemplateColumns: '300px 1fr' }}>
        <aside className="py-2 px-0 border-r overflow-auto" >
          <div style={{
 fontSize: 11,
 letterSpacing: '0.14em' }} className="pt-2 pb-1 px-4 text-ink-4 uppercase" >
            Documents
          </div>
          {projectDocs.map(d => {
            const open = openDocId === d.id;
            const hp = d.current / Math.max(1, d.current + d.drifted + d.broken);
            return (
              <button key={d.id} onClick={() => setOpenDocId(d.id)}
 style={{
 background: open ? 'var(--paper-2)' : 'transparent',
 borderLeft: open ? '2px solid var(--accent)'
 : '2px solid transparent' }} className="py-2 px-4 w-full text-left cursor-pointer" >
                <div className="mono" style={{ fontSize: 11,
                              color: open ? 'var(--ink)' : 'var(--ink-2)' }}>
                  {d.title}
                </div>
                <div className="gap-2 mt-1 flex items-center" >
                  <HealthBar current={d.current} drifted={d.drifted} broken={d.broken} compact/>
                  <span className="mono" style={{ fontSize: 11,
                                color: hp >= 0.9 ? 'var(--success)' : 'var(--warning)' }}>
                    {Math.round(hp*100)}%
                  </span>
                  <span className="flex-1" />
                  <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                    {d.links} links
                  </span>
                </div>
              </button>
            );
          })}
        </aside>

        <main className="pt-6 pb-8 px-8 overflow-auto" >
          {doc && <DocDetail doc={doc}/>}
        </main>
      </div>
    </div>
  );
}

function HealthBar({ current, drifted, broken, compact }) {
  const tot = current + drifted + broken || 1;
  const w = compact ? 60 : '100%';
  const h = compact ? 3 : 6;
  return (
    <div className="flex overflow-hidden" style={{ width: w, height: h,
 borderRadius: 2,
 background: 'var(--edge)' }}>
      <div className="bg-success" style={{ width: `${(current/tot)*100}%` }}/>
      <div className="bg-warning" style={{ width: `${(drifted/tot)*100}%` }}/>
      <div className="bg-accent" style={{ width: `${(broken/tot)*100}%` }}/>
    </div>
  );
}

function DocDetail({ doc }) {
  const refs = doc.references.length ? doc.references : [
    { id: "x", lineRef: "—", quote: "(no references indexed yet)",
      target: { symbol: "—", path: "—" }, status: "current",
      expected: "", actual: "", reason: "" }
  ];
  const drifted = refs.filter(r => r.status === "drifted").length;
  const broken  = refs.filter(r => r.status === "broken").length;

  return (
    <div style={{ maxWidth: 800 }}>
      <div className="gap-3 mb-1 flex items-baseline" >
        <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>document</span>
        <span className="flex-1" />
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          last checked {doc.lastChecked} · modified {doc.lastModified}
        </span>
      </div>
      <h2 className="display mono m-0 font-normal text-ink" style={{
 fontSize: 22, letterSpacing: 0
 }}>
        {doc.path}
      </h2>

      <div className="gap-6 mb-6 mt-3 py-4 px-0 flex border-b" >
        <DocStat n={doc.links} l="references"/>
        <DocStat n={doc.current} l="current" tone="jade"/>
        <DocStat n={doc.drifted} l="drifted" tone="amber"/>
        <DocStat n={doc.broken}  l="broken"  tone="shu"/>
        <span className="flex-1" />
        {(drifted + broken) > 0 && (
          <button style={{
 fontSize: 13, borderRadius: 5 }} className="py-2 px-4 gap-2 bg-ink text-paper border-0 cursor-pointer inline-flex items-center" >
            <span className="kanji text-accent" style={{ fontSize: 13 }}>直</span>
            Fix all {drifted + broken} drift items →
          </button>
        )}
      </div>

      {/* Reference list */}
      <div className="gap-2 flex flex-col" >
        {doc.references.map(r => <ReferenceRow key={r.id} r={r}/>)}
        {doc.references.length === 0 && (
          <div style={{ fontSize: 13 }} className="py-8 px-4 text-center text-ink-4 italic" >
            All {doc.links} references current.
          </div>
        )}
      </div>
    </div>
  );
}

function DocStat({ n, l, tone }) {
  const c = tone === "jade" ? 'var(--success)' :
            tone === "amber" ? 'var(--warning)' :
            tone === "shu" ? 'var(--accent)' : 'var(--ink)';
  return (
    <div>
      <div className="display font-light" style={{ fontSize: 22, color: c, lineHeight: 1.1 }}>
        {n}
      </div>
      <div style={{
 fontSize: 11,
 letterSpacing: '0.1em' }} className="mt-1 text-ink-3 uppercase" >{l}</div>
    </div>
  );
}

function ReferenceRow({ r }) {
  const tone = r.status === "current" ? 'var(--success)' :
               r.status === "drifted" ? 'var(--warning)' : 'var(--accent)';
  const [open, setOpen] = dtS(r.status !== "current");

  return (
    <article style={{
 borderLeft: `2px solid ${tone}`,
 borderRadius: 5
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
      <div className="gap-2 flex items-center" >
        <span className="uppercase font-medium" style={{ fontSize: 11, letterSpacing: '0.14em', color: tone }}>
          {r.status}
        </span>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {r.lineRef}
        </span>
        <span className="text-ink-2 italic flex-1 overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 13 }}>
          "{r.quote}"
        </span>
        {r.status !== "current" && (
          <button style={{
 fontSize: 11, borderRadius: 4 }} className="py-1 px-3 bg-ink text-paper border-0 cursor-pointer whitespace-nowrap" >
            Fix drift →
          </button>
        )}
      </div>
      <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
        →  <span className="text-ink" >{r.target.symbol}</span>
        <span className="text-ink-4" >  ·  {r.target.path}</span>
      </div>

      {r.status !== "current" && (
        <div style={{ borderRadius: 4 }} className="mt-2 py-2 px-3 bg-paper border border-paper-edge" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-4 uppercase" >signature delta</div>
          <div className="mono text-ink-2" style={{ fontSize: 11,
 lineHeight: 1.55 }}>
            <div className="text-ink-3" >doc says:  {r.expected}</div>
            <div className="text-ink" >code is:   {r.actual}</div>
            {r.diff && <div style={{ color: tone }} className="mt-1" >Δ          {r.diff}</div>}
          </div>
          {r.reason && (
            <div style={{
 fontSize: 11,
 lineHeight: 1.55
 }} className="mt-2 text-ink-2" >{r.reason}</div>
          )}
        </div>
      )}
    </article>
  );
}

window.ObsTraceability = ObsTraceability;
