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

      <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 24 }}>
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>巻</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            Observatory · Document traceability
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>
            Where the docs and the code disagree.
          </h1>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            Every doc-to-symbol link, checked nightly. Drift becomes visible
            here before someone reads stale docs and writes the wrong thing.
          </p>
        </div>
      </div>

      {/* Project rollup strip */}
      <div style={{ padding: '12px 32px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 0 }}>
        {T.projectRollup.map(p => {
          const on = project === p.id;
          const tot = p.current + p.drifted + p.broken;
          return (
            <button key={p.id}
                    onClick={() => { setProject(p.id);
                                     const fd = T.docs.find(d => d.project === p.id);
                                     setOpenDocId(fd?.id); }}
                    style={{ flex: 1, textAlign: 'left',
                              padding: '8px 16px',
                              background: on ? 'var(--paper-2)' : 'transparent',
                              border: 'var(--hairline)', borderRadius: 6,
                              cursor: 'pointer', marginRight: 8 }}>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 4 }}>
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
              <div style={{ display: 'flex', gap: 12, marginTop: 4,
                             fontSize: 11, color: 'var(--ink-3)' }}>
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
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto', padding: '8px 0' }}>
          <div style={{ padding: '8px 16px 4px', fontSize: 11,
                         letterSpacing: '0.14em', color: 'var(--ink-4)',
                         textTransform: 'uppercase' }}>
            Documents
          </div>
          {projectDocs.map(d => {
            const open = openDocId === d.id;
            const hp = d.current / Math.max(1, d.current + d.drifted + d.broken);
            return (
              <button key={d.id} onClick={() => setOpenDocId(d.id)}
                      style={{ width: '100%', textAlign: 'left',
                                padding: '8px 16px',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                  : '2px solid transparent',
                                cursor: 'pointer' }}>
                <div className="mono" style={{ fontSize: 11,
                              color: open ? 'var(--ink)' : 'var(--ink-2)' }}>
                  {d.title}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 4 }}>
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

        <main style={{ overflow: 'auto', padding: '24px 32px 32px' }}>
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
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 4 }}>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase' }}>document</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          last checked {doc.lastChecked} · modified {doc.lastModified}
        </span>
      </div>
      <h2 className="display mono" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                              color: 'var(--ink)', letterSpacing: 0 }}>
        {doc.path}
      </h2>

      <div style={{ display: 'flex', gap: 24, padding: '16px 0',
                     borderBottom: 'var(--hairline)', marginBottom: 24, marginTop: 12 }}>
        <DocStat n={doc.links} l="references"/>
        <DocStat n={doc.current} l="current" tone="jade"/>
        <DocStat n={doc.drifted} l="drifted" tone="amber"/>
        <DocStat n={doc.broken}  l="broken"  tone="shu"/>
        <span style={{ flex: 1 }}/>
        {(drifted + broken) > 0 && (
          <button style={{ padding: '8px 16px', fontSize: 13,
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 5, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>直</span>
            Fix all {drifted + broken} drift items →
          </button>
        )}
      </div>

      {/* Reference list */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {doc.references.map(r => <ReferenceRow key={r.id} r={r}/>)}
        {doc.references.length === 0 && (
          <div style={{ padding: '32px 16px', textAlign: 'center', fontSize: 13,
                         color: 'var(--ink-4)', fontStyle: 'italic' }}>
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
      <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4,
                     letterSpacing: '0.1em', textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

function ReferenceRow({ r }) {
  const tone = r.status === "current" ? 'var(--success)' :
               r.status === "drifted" ? 'var(--warning)' : 'var(--accent)';
  const [open, setOpen] = dtS(r.status !== "current");

  return (
    <article style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${tone}`,
                       borderRadius: 5, padding: '12px 16px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
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
          <button style={{ fontSize: 11, padding: '4px 12px',
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 4, cursor: 'pointer',
                            whiteSpace: 'nowrap' }}>
            Fix drift →
          </button>
        )}
      </div>
      <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
        →  <span style={{ color: 'var(--ink)' }}>{r.target.symbol}</span>
        <span style={{ color: 'var(--ink-4)' }}>  ·  {r.target.path}</span>
      </div>

      {r.status !== "current" && (
        <div style={{ marginTop: 8, padding: '8px 12px',
                       background: 'var(--paper)', borderRadius: 4,
                       border: 'var(--hairline)' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                         textTransform: 'uppercase', marginBottom: 4 }}>signature delta</div>
          <div className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
                                          lineHeight: 1.55 }}>
            <div style={{ color: 'var(--ink-3)' }}>doc says:  {r.expected}</div>
            <div style={{ color: 'var(--ink)' }}>code is:   {r.actual}</div>
            {r.diff && <div style={{ color: tone, marginTop: 4 }}>Δ          {r.diff}</div>}
          </div>
          {r.reason && (
            <div style={{ fontSize: 11, color: 'var(--ink-2)', marginTop: 8,
                           lineHeight: 1.55 }}>{r.reason}</div>
          )}
        </div>
      )}
    </article>
  );
}

window.ObsTraceability = ObsTraceability;
