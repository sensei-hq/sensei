// Document traceability — drift visibility before it causes harm.
//
// Per-project rollup → drill into a doc → each reference with status.
// Drifted/broken references surface a "fix drift" prompt that ships off
// to the assistant.

const { useState: dtS } = React;

function ObsTraceability() {
  const T = window.UPGRADES.trace;
  const [project, setProject] = dtS(T.projectRollup[0].id);
  const [openDocId, setOpenDocId] = dtS(T.docs.find(d => d.project === T.projectRollup[0].id)?.id);

  const projectDocs = T.docs.filter(d => d.project === project);
  const doc = T.docs.find(d => d.id === openDocId) || projectDocs[0];

  return (
    <div className="sensei" data-screen-label="Observatory · Traceability"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>巻</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Observatory · Document traceability
          </div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            Where the docs and the code disagree.
          </h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >
            Every doc-to-symbol link, checked nightly. Drift becomes visible
            here before someone reads stale docs and writes the wrong thing.
          </p>
        </div>
      </div>

      {/* Project rollup strip */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex'
}} className="py-3 px-6 gap-0" >
        {T.projectRollup.map(p => {
          const on = project === p.id;
          const tot = p.current + p.drifted + p.broken;
          return (
            <button key={p.id}
                    onClick={() => { setProject(p.id);
                                     const fd = T.docs.find(d => d.project === p.id);
                                     setOpenDocId(fd?.id); }}
                    style={{
 flex: 1, textAlign: 'left',
                              background: on ? 'var(--paper-2)' : 'transparent',
                              border: 'var(--hairline)', borderRadius: 6,
                              cursor: 'pointer'
}} className="py-2 px-4 mr-2" >
              <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-1" >
                <span className="kanji" style={{ fontSize: 13,
                              color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{p.kanji}</span>
                <span className="mono" style={{ fontSize: 11,
                              color: on ? 'var(--ink)' : 'var(--ink-2)' }}>{p.name}</span>
                <span style={{ flex: 1 }}/>
                <span className="mono" style={{ fontSize: 11,
                              color: p.healthPct >= 0.9 ? 'var(--success)' :
                                     p.healthPct >= 0.8 ? 'var(--ink-2)' : 'var(--warning)' }}>
                  {Math.round(p.healthPct*100)}%
                </span>
              </div>
              <HealthBar current={p.current} drifted={p.drifted} broken={p.broken}/>
              <div style={{
 display: 'flex',
                             fontSize: 11, color: 'var(--ink-3)'
}} className="gap-3 mt-1" >
                <span>{p.docs} docs</span>
                <span>{p.links} links</span>
                <span style={{ color: 'var(--warning)' }}>{p.drifted} drifted</span>
                <span style={{ color: 'var(--accent)' }}>{p.broken} broken</span>
              </div>
            </button>
          );
        })}
      </div>

      <div style={{ flex: 1, display: 'grid',
                     gridTemplateColumns: '300px 1fr',
                     minHeight: 0 }}>
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto' }} className="py-2 px-0" >
          <div style={{
 fontSize: 11,
                         letterSpacing: '0.14em', color: 'var(--ink-4)',
                         textTransform: 'uppercase'
}} className="pt-2 pb-1 px-4" >
            Documents
          </div>
          {projectDocs.map(d => {
            const open = openDocId === d.id;
            const hp = d.current / Math.max(1, d.current + d.drifted + d.broken);
            return (
              <button key={d.id} onClick={() => setOpenDocId(d.id)}
                      style={{
 width: '100%', textAlign: 'left',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                  : '2px solid transparent',
                                cursor: 'pointer'
}} className="py-2 px-4" >
                <div className="mono" style={{ fontSize: 11,
                              color: open ? 'var(--ink)' : 'var(--ink-2)' }}>
                  {d.title}
                </div>
                <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mt-1" >
                  <HealthBar current={d.current} drifted={d.drifted} broken={d.broken} compact/>
                  <span className="mono" style={{ fontSize: 11,
                                color: hp >= 0.9 ? 'var(--success)' : 'var(--warning)' }}>
                    {Math.round(hp*100)}%
                  </span>
                  <span style={{ flex: 1 }}/>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                    {d.links} links
                  </span>
                </div>
              </button>
            );
          })}
        </aside>

        <main style={{ overflow: 'auto' }} className="pt-5 pb-6 px-6" >
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
    <div style={{ width: w, height: h, display: 'flex',
                   borderRadius: 2, overflow: 'hidden',
                   background: 'var(--edge)' }}>
      <div style={{ width: `${(current/tot)*100}%`, background: 'var(--success)' }}/>
      <div style={{ width: `${(drifted/tot)*100}%`, background: 'var(--warning)' }}/>
      <div style={{ width: `${(broken/tot)*100}%`, background: 'var(--accent)' }}/>
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
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-3 mb-1" >
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase' }}>document</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          last checked {doc.lastChecked} · modified {doc.lastModified}
        </span>
      </div>
      <h2 className="display mono m-0" style={{
 fontSize: 22, fontWeight: 400,
                                              color: 'var(--ink)', letterSpacing: 0
}}>
        {doc.path}
      </h2>

      <div style={{
 display: 'flex',
                     borderBottom: 'var(--hairline)'
}} className="gap-5 mb-5 mt-3 py-4 px-0" >
        <DocStat n={doc.links} l="references"/>
        <DocStat n={doc.current} l="current" tone="jade"/>
        <DocStat n={doc.drifted} l="drifted" tone="amber"/>
        <DocStat n={doc.broken}  l="broken"  tone="shu"/>
        <span style={{ flex: 1 }}/>
        {(drifted + broken) > 0 && (
          <button style={{
 fontSize: 13,
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 5, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center'
}} className="py-2 px-4 gap-2" >
            <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>直</span>
            Fix all {drifted + broken} drift items →
          </button>
        )}
      </div>

      {/* Reference list */}
      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
        {doc.references.map(r => <ReferenceRow key={r.id} r={r}/>)}
        {doc.references.length === 0 && (
          <div style={{
 textAlign: 'center', fontSize: 13,
                         color: 'var(--ink-4)', fontStyle: 'italic'
}} className="py-6 px-4" >
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
      <div className="display" style={{ fontSize: 22, fontWeight: 300, color: c, lineHeight: 1.1 }}>
        {n}
      </div>
      <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                     letterSpacing: '0.1em', textTransform: 'uppercase'
}} className="mt-1" >{l}</div>
    </div>
  );
}

function ReferenceRow({ r }) {
  const tone = r.status === "current" ? 'var(--success)' :
               r.status === "drifted" ? 'var(--warning)' : 'var(--accent)';
  const [open, setOpen] = dtS(r.status !== "current");

  return (
    <article style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${tone}`,
                       borderRadius: 5
}} className="py-3 px-4" >
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: tone,
                        textTransform: 'uppercase', fontWeight: 500 }}>
          {r.status}
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {r.lineRef}
        </span>
        <span style={{ fontSize: 13, color: 'var(--ink-2)', fontStyle: 'italic',
                        flex: 1, overflow: 'hidden', textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap' }}>
          "{r.quote}"
        </span>
        {r.status !== "current" && (
          <button style={{
 fontSize: 11,
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 4, cursor: 'pointer',
                            whiteSpace: 'nowrap'
}} className="py-1 px-3" >
            Fix drift →
          </button>
        )}
      </div>
      <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
        →  <span style={{ color: 'var(--ink)' }}>{r.target.symbol}</span>
        <span style={{ color: 'var(--ink-4)' }}>  ·  {r.target.path}</span>
      </div>

      {r.status !== "current" && (
        <div style={{
                       background: 'var(--paper)', borderRadius: 4,
                       border: 'var(--hairline)'
}} className="mt-2 py-2 px-3" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                         textTransform: 'uppercase'
}} className="mb-1" >signature delta</div>
          <div className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
                                          lineHeight: 1.55 }}>
            <div style={{ color: 'var(--ink-3)' }}>doc says:  {r.expected}</div>
            <div style={{ color: 'var(--ink)' }}>code is:   {r.actual}</div>
            {r.diff && <div style={{ color: tone }} className="mt-1" >Δ          {r.diff}</div>}
          </div>
          {r.reason && (
            <div style={{
 fontSize: 11, color: 'var(--ink-2)',
                           lineHeight: 1.55
}} className="mt-2" >{r.reason}</div>
          )}
        </div>
      )}
    </article>
  );
}

window.ObsTraceability = ObsTraceability;
