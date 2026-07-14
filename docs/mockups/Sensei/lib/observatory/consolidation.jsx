// Memory consolidation review.
//
// Sensei has spotted overlapping memories. Each proposal shows the source
// memories and the merged result the system would create. The user
// accepts (sources archived → merged created → past_memories preserves
// the audit trail) or keeps them separate.

const { useState: cnS } = React;

function ObsConsolidation() {
  const items = window.UPGRADES.consolidations;
  const [openId, setOpen] = cnS(items[0].id);
  const [decisions, setDecisions] = cnS({});  // id → "merged" | "kept"
  const item = items.find(x => x.id === openId) || items[0];

  const decide = (id, choice) => setDecisions({ ...decisions, [id]: choice });

  return (
    <div className="sensei" data-screen-label="Observatory · Memory consolidation"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* Hero */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>結</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Memories · consolidation
          </div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            Three pairs of memories say nearly the same thing.
          </h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >
            Merging keeps the canonical statement, combines evidence, and
            archives the originals. The audit trail is preserved in
            <span className="mono"> history.past_memories </span>so nothing
            is lost — only deduplicated.
          </p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex'
}} className="gap-5 pl-5" >
          <UgMini n={items.length} l="proposals"/>
          <UgMini n={items.reduce((s,x)=>s+x.sources.length,0)} l="memories"/>
          <UgMini n={`-${items.reduce((s,x)=>s+x.sources.length-1,0)}`}
                  l="net reduction" mono accent/>
        </div>
      </div>

      <div style={{ flex: 1, display: 'grid',
                     gridTemplateColumns: '320px 1fr',
                     minHeight: 0 }}>
        {/* Proposal list */}
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto' }} className="py-2 px-0" >
          {items.map(p => {
            const open = openId === p.id;
            const d = decisions[p.id];
            return (
              <button key={p.id} onClick={() => setOpen(p.id)}
                      style={{
 width: '100%', textAlign: 'left',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                 : '2px solid transparent',
                                cursor: 'pointer'
}} className="py-3 px-4" >
                <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-1" >
                  <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>結</span>
                  <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                                  textTransform: 'uppercase' }}>
                    {p.sources.length} → 1
                  </span>
                  <span style={{ flex: 1 }}/>
                  {d && (
                    <span style={{ fontSize: 11, letterSpacing: '0.1em',
                                    textTransform: 'uppercase',
                                    color: d === "merged" ? 'var(--success)' : 'var(--ink-3)' }}>
                      {d}
                    </span>
                  )}
                </div>
                <div style={{
 fontSize: 13,
                               color: open ? 'var(--ink)' : 'var(--ink-2)',
                               lineHeight: 1.4, fontWeight: 500
}} className="mb-1" >
                  {p.title}
                </div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
                              lineHeight: 1.5 }}>
                  {p.sourceIds.join(" + ")}
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main style={{ overflow: 'auto' }} className="pt-5 pb-6 px-6" >
          {item && <ConsolidationDetail
                      p={item}
                      decision={decisions[item.id]}
                      onDecide={(c) => decide(item.id, c)}/>}
        </main>
      </div>
    </div>
  );
}

function ConsolidationDetail({ p, decision, onDecide }) {
  return (
    <div style={{ maxWidth: 920 }}>
      <div style={{
 display: 'flex', alignItems: 'center',
                     fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase'
}} className="gap-3 mb-3" >
        <span>Consolidation proposal</span>
        <Sep/>
        <span className="mono" style={{ letterSpacing: 0 }}>{p.id}</span>
      </div>
      <h2 className="display mt-0 mb-2" style={{
 fontSize: 28, fontWeight: 300,
                                        lineHeight: 1.2, letterSpacing: '-0.015em', color: 'var(--ink)'
}}>
        {p.title}
      </h2>
      <p style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.65, maxWidth: 720
}} className="mt-0 mb-5" >{p.reason}</p>

      {/* Sources column → Merged column visualization */}
      <div style={{
 display: 'grid', gridTemplateColumns: '1fr 24px 1fr', alignItems: 'stretch'
}} className="mb-5 gap-0" >

        {/* Sources */}
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-3" >
            Source memories ({p.sources.length})
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
            {p.sources.map(s => <MemorySnippet key={s.id} m={s} dim/>)}
          </div>
        </div>

        {/* Arrow column */}
        <div style={{
 display: 'flex', flexDirection: 'column',
                       alignItems: 'center', justifyContent: 'center'
}} className="px-1" >
          <div style={{ width: 1, flex: 1, background: 'var(--edge)' }}/>
          <span className="kanji my-2 mx-0" style={{
 fontSize: 17, color: 'var(--accent)'
}}>→</span>
          <div style={{ width: 1, flex: 1, background: 'var(--edge)' }}/>
        </div>

        {/* Proposed merged */}
        <div>
          <div style={{
 display: 'flex', alignItems: 'center'
}} className="mb-3 gap-1" >
            <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--accent)',
                            textTransform: 'uppercase', fontWeight: 500 }}>
              Proposed merged memory
            </span>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>新</span>
          </div>
          <MergedMemory m={p.proposed}/>
        </div>
      </div>

      {/* Diff strip — what changes about evidence + strength */}
      <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', background: 'var(--edge)',
                     borderRadius: 6, overflow: 'hidden'
}} className="gap-1 mb-5" >
        <DiffStat label="Strength"
                  before={Math.max(...p.sources.map(s => s.strength))}
                  after={p.proposed.strength}
                  delta={p.proposed.strength - Math.max(...p.sources.map(s => s.strength))}/>
        <DiffStat label="Evidence sessions"
                  before={p.sources.reduce((s,x)=>s+x.evidence.length,0)}
                  after={p.proposed.evidence.length}
                  same/>
        <DiffStat label="Memories on disk"
                  before={p.sources.length}
                  after={1}
                  delta={1 - p.sources.length}
                  positiveLow/>
        <DiffStat label="Violations carried"
                  before={p.sources.reduce((s,x)=>s+x.violated,0)}
                  after={p.proposed.violations}
                  same/>
      </div>

      {/* Actions */}
      {!decision ? (
        <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 pt-1" >
          <button onClick={() => onDecide("merged")}
                  style={{
 fontSize: 13,
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center'
}} className="py-2 px-4 gap-2" >
            <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>結</span>
            Accept · merge & archive originals
          </button>
          <button onClick={() => onDecide("kept")}
                  style={{
 fontSize: 13,
                            background: 'var(--paper-2)', color: 'var(--ink)',
                            border: 'var(--hairline)', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center'
}} className="py-2 px-4 gap-2" >
            <span className="kanji" style={{ fontSize: 13, color: 'var(--ink-3)' }}>別</span>
            Keep separate
          </button>
          <span style={{ flex: 1 }}/>
          <FlatBtn glyph="編" label="Edit before merging"/>
        </div>
      ) : (
        <div style={{
                       background: decision === "merged" ? 'var(--success-soft)'
                                                          : 'var(--paper-2)',
                       border: 'var(--hairline)',
                       borderLeft: `2px solid ${decision === "merged" ? 'var(--success)' : 'var(--ink-3)'}`,
                       borderRadius: 6,
                       display: 'flex', alignItems: 'center'
}} className="py-3 px-4 gap-3" >
          <span className="kanji" style={{ fontSize: 15,
                        color: decision === "merged" ? 'var(--success)' : 'var(--ink-3)' }}>
            {decision === "merged" ? "結" : "別"}
          </span>
          <div style={{ flex: 1, fontSize: 13, color: 'var(--ink)' }}>
            {decision === "merged"
              ? <>Merged. <span className="mono" style={{ color: 'var(--ink-3)' }}>
                    {p.sourceIds.join(", ")}
                  </span> archived to history.past_memories.</>
              : <>Kept separate. Sensei will not surface this proposal again
                  unless new evidence accumulates.</>}
          </div>
          <button onClick={() => onDecide(null)}
                  style={{
 fontSize: 11, color: 'var(--ink-3)',
                            background: 'transparent', border: 'var(--hairline)',
                            borderRadius: 4, cursor: 'pointer'
}} className="py-1 px-2" >undo</button>
        </div>
      )}
    </div>
  );
}

function MemorySnippet({ m, dim }) {
  return (
    <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 5,
                   opacity: dim ? 0.78 : 1
}} className="py-3 px-3" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-1" >
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{m.id}</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
          str {m.strength}
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {m.evidence.length} ev
        </span>
        {m.violated > 0 && (
          <span className="mono" style={{ fontSize: 11, color: 'var(--warning)' }}>
            {m.violated}× violated
          </span>
        )}
      </div>
      <div style={{
 fontSize: 13, color: 'var(--ink)', lineHeight: 1.5
}} className="mb-1" >{m.what}</div>
      <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55,
                     fontStyle: 'italic' }}>{m.because}</div>
    </div>
  );
}

function MergedMemory({ m }) {
  return (
    <div style={{
 background: 'var(--paper)',
                   border: '1px solid var(--accent)',
                   borderRadius: 5,
                   boxShadow: '0 1px 0 var(--edge)'
}} className="py-3 px-3" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-1" >
        <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>m-merged-pending</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
          str {m.strength}
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {m.evidence.length} ev
        </span>
      </div>
      <div style={{
 fontSize: 13, color: 'var(--ink)', lineHeight: 1.5, fontWeight: 500
}} className="mb-1" >{m.what}</div>
      <div style={{
 fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55,
                     fontStyle: 'italic'
}} className="mb-2" >{m.because}</div>
      <div style={{
 display: 'flex', flexWrap: 'wrap', borderTop: 'var(--hairline)'
}} className="gap-1 pt-2" >
        <ScopeChip s={m.scope}/>
        {m.scope.filePatterns?.map(g => (
          <span key={g} className="mono py-1 px-1" style={{
 fontSize: 11,
                       borderRadius: 3, background: 'var(--paper-2)',
                       color: 'var(--ink-3)'
}}>{g}</span>
        ))}
      </div>
    </div>
  );
}

function ScopeChip({ s }) {
  const txt =
    s.level === "global"  ? "global" :
    s.level === "stack"   ? `stack · ${s.stack?.join(" + ") || "?"}` :
    s.level === "project" ? `project · ${s.project}${s.modules ? ` · ${s.modules.join(",")}` : ""}` :
    s.level || "scoped";
  return (
    <span className="mono py-1 px-2" style={{
 fontSize: 11,
                  borderRadius: 3,
                  background: 'var(--paper-2)',
                  color: 'var(--ink-2)',
                  letterSpacing: 0
}}>
      {txt}
    </span>
  );
}

function DiffStat({ label, before, after, delta, same, positiveLow }) {
  const positive = same ? null : positiveLow ? delta < 0 : delta > 0;
  const color = same ? 'var(--ink-3)' :
                positive ? 'var(--success)' :
                positive === false ? 'var(--accent)' : 'var(--ink-3)';
  return (
    <div style={{ background: 'var(--paper-2)' }} className="py-3 px-3" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase'
}} className="mb-1" >{label}</div>
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{before}</span>
        <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>→</span>
        <span className="display" style={{ fontSize: 17, fontWeight: 400,
                      color: 'var(--ink)' }}>{after}</span>
        {!same && (
          <span className="mono ml-auto" style={{ fontSize: 11, color }}>
            {delta > 0 ? "+" : ""}{delta}
          </span>
        )}
      </div>
    </div>
  );
}

window.ObsConsolidation = ObsConsolidation;
