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
      <div style={{ padding: '22px 36px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 22 }}>
        <div className="kanji" style={{ fontSize: 42, color: 'var(--shu)', lineHeight: 1 }}>結</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 5 }}>
            Memories · consolidation
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--sumi)' }}>
            Three pairs of memories say nearly the same thing.
          </h1>
          <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            Merging keeps the canonical statement, combines evidence, and
            archives the originals. The audit trail is preserved in
            <span className="mono"> history.past_memories </span>so nothing
            is lost — only deduplicated.
          </p>
        </div>
        <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 22 }}>
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
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto', padding: '8px 0' }}>
          {items.map(p => {
            const open = openId === p.id;
            const d = decisions[p.id];
            return (
              <button key={p.id} onClick={() => setOpen(p.id)}
                      style={{ width: '100%', textAlign: 'left',
                                padding: '14px 18px',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--shu)'
                                                 : '2px solid transparent',
                                cursor: 'pointer' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                  <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>結</span>
                  <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                                  textTransform: 'uppercase' }}>
                    {p.sources.length} → 1
                  </span>
                  <span style={{ flex: 1 }}/>
                  {d && (
                    <span style={{ fontSize: 9.5, letterSpacing: '0.1em',
                                    textTransform: 'uppercase',
                                    color: d === "merged" ? 'var(--jade)' : 'var(--sumi-3)' }}>
                      {d}
                    </span>
                  )}
                </div>
                <div style={{ fontSize: 12.5,
                               color: open ? 'var(--sumi)' : 'var(--sumi-2)',
                               lineHeight: 1.4, fontWeight: 500, marginBottom: 6 }}>
                  {p.title}
                </div>
                <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                              lineHeight: 1.5 }}>
                  {p.sourceIds.join(" + ")}
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main style={{ overflow: 'auto', padding: '24px 40px 36px' }}>
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
      <div style={{ display: 'flex', alignItems: 'center', gap: 12,
                     fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase', marginBottom: 12 }}>
        <span>Consolidation proposal</span>
        <Sep/>
        <span className="mono" style={{ letterSpacing: 0 }}>{p.id}</span>
      </div>
      <h2 className="display" style={{ fontSize: 26, fontWeight: 300,
                                        lineHeight: 1.2, letterSpacing: '-0.015em',
                                        margin: '0 0 10px', color: 'var(--sumi)' }}>
        {p.title}
      </h2>
      <p style={{ fontSize: 13.5, color: 'var(--sumi-2)', lineHeight: 1.65,
                   margin: '0 0 26px', maxWidth: 720 }}>{p.reason}</p>

      {/* Sources column → Merged column visualization */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 24px 1fr',
                     gap: 0, alignItems: 'stretch', marginBottom: 24 }}>

        {/* Sources */}
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 12 }}>
            Source memories ({p.sources.length})
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {p.sources.map(s => <MemorySnippet key={s.id} m={s} dim/>)}
          </div>
        </div>

        {/* Arrow column */}
        <div style={{ display: 'flex', flexDirection: 'column',
                       alignItems: 'center', justifyContent: 'center',
                       padding: '0 4px' }}>
          <div style={{ width: 1, flex: 1, background: 'var(--paper-edge)' }}/>
          <span className="kanji" style={{ fontSize: 18, color: 'var(--shu)',
                        margin: '8px 0' }}>→</span>
          <div style={{ width: 1, flex: 1, background: 'var(--paper-edge)' }}/>
        </div>

        {/* Proposed merged */}
        <div>
          <div style={{ display: 'flex', alignItems: 'center',
                         marginBottom: 12, gap: 6 }}>
            <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--shu)',
                            textTransform: 'uppercase', fontWeight: 500 }}>
              Proposed merged memory
            </span>
            <span className="kanji" style={{ fontSize: 12, color: 'var(--shu)' }}>新</span>
          </div>
          <MergedMemory m={p.proposed}/>
        </div>
      </div>

      {/* Diff strip — what changes about evidence + strength */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                     gap: 1, background: 'var(--paper-edge)',
                     borderRadius: 6, overflow: 'hidden', marginBottom: 22 }}>
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
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', paddingTop: 4 }}>
          <button onClick={() => onDecide("merged")}
                  style={{ padding: '9px 18px', fontSize: 13,
                            background: 'var(--sumi)', color: 'var(--paper)',
                            border: 'none', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>結</span>
            Accept · merge & archive originals
          </button>
          <button onClick={() => onDecide("kept")}
                  style={{ padding: '9px 16px', fontSize: 12.5,
                            background: 'var(--paper-2)', color: 'var(--sumi)',
                            border: 'var(--hairline)', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center', gap: 7 }}>
            <span className="kanji" style={{ fontSize: 12.5, color: 'var(--sumi-3)' }}>別</span>
            Keep separate
          </button>
          <span style={{ flex: 1 }}/>
          <FlatBtn glyph="編" label="Edit before merging"/>
        </div>
      ) : (
        <div style={{ padding: '14px 18px',
                       background: decision === "merged" ? 'var(--jade-soft)'
                                                          : 'var(--paper-2)',
                       border: 'var(--hairline)',
                       borderLeft: `2px solid ${decision === "merged" ? 'var(--jade)' : 'var(--sumi-3)'}`,
                       borderRadius: 6,
                       display: 'flex', alignItems: 'center', gap: 12 }}>
          <span className="kanji" style={{ fontSize: 16,
                        color: decision === "merged" ? 'var(--jade)' : 'var(--sumi-3)' }}>
            {decision === "merged" ? "結" : "別"}
          </span>
          <div style={{ flex: 1, fontSize: 12.5, color: 'var(--sumi)' }}>
            {decision === "merged"
              ? <>Merged. <span className="mono" style={{ color: 'var(--sumi-3)' }}>
                    {p.sourceIds.join(", ")}
                  </span> archived to history.past_memories.</>
              : <>Kept separate. Sensei will not surface this proposal again
                  unless new evidence accumulates.</>}
          </div>
          <button onClick={() => onDecide(null)}
                  style={{ fontSize: 11, color: 'var(--sumi-3)',
                            padding: '4px 10px',
                            background: 'transparent', border: 'var(--hairline)',
                            borderRadius: 4, cursor: 'pointer' }}>undo</button>
        </div>
      )}
    </div>
  );
}

function MemorySnippet({ m, dim }) {
  return (
    <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 5, padding: '12px 14px',
                   opacity: dim ? 0.78 : 1 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 6 }}>
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>{m.id}</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 10, color: 'var(--shu)' }}>
          str {m.strength}
        </span>
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
          {m.evidence.length} ev
        </span>
        {m.violated > 0 && (
          <span className="mono" style={{ fontSize: 10, color: 'var(--amber)' }}>
            {m.violated}× violated
          </span>
        )}
      </div>
      <div style={{ fontSize: 12.5, color: 'var(--sumi)', lineHeight: 1.5,
                     marginBottom: 5 }}>{m.what}</div>
      <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55,
                     fontStyle: 'italic' }}>{m.because}</div>
    </div>
  );
}

function MergedMemory({ m }) {
  return (
    <div style={{ background: 'var(--paper)',
                   border: '1px solid var(--shu)',
                   borderRadius: 5, padding: '12px 14px',
                   boxShadow: '0 1px 0 var(--paper-edge)' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 6 }}>
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--shu)' }}>m-merged-pending</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 10, color: 'var(--shu)' }}>
          str {m.strength}
        </span>
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
          {m.evidence.length} ev
        </span>
      </div>
      <div style={{ fontSize: 12.5, color: 'var(--sumi)', lineHeight: 1.5,
                     marginBottom: 5, fontWeight: 500 }}>{m.what}</div>
      <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55,
                     fontStyle: 'italic', marginBottom: 8 }}>{m.because}</div>
      <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap',
                     paddingTop: 8, borderTop: 'var(--hairline)' }}>
        <ScopeChip s={m.scope}/>
        {m.scope.filePatterns?.map(g => (
          <span key={g} className="mono" style={{ fontSize: 9.5, padding: '2px 6px',
                       borderRadius: 3, background: 'var(--paper-2)',
                       color: 'var(--sumi-3)' }}>{g}</span>
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
    <span className="mono" style={{ fontSize: 9.5, padding: '2px 7px',
                  borderRadius: 3,
                  background: 'var(--paper-2)',
                  color: 'var(--sumi-2)',
                  letterSpacing: 0 }}>
      {txt}
    </span>
  );
}

function DiffStat({ label, before, after, delta, same, positiveLow }) {
  const positive = same ? null : positiveLow ? delta < 0 : delta > 0;
  const color = same ? 'var(--sumi-3)' :
                positive ? 'var(--jade)' :
                positive === false ? 'var(--shu)' : 'var(--sumi-3)';
  return (
    <div style={{ background: 'var(--paper-2)', padding: '12px 14px' }}>
      <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-4)',
                     textTransform: 'uppercase', marginBottom: 6 }}>{label}</div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{before}</span>
        <span style={{ fontSize: 11, color: 'var(--sumi-4)' }}>→</span>
        <span className="display" style={{ fontSize: 17, fontWeight: 400,
                      color: 'var(--sumi)' }}>{after}</span>
        {!same && (
          <span className="mono" style={{ fontSize: 11, color, marginLeft: 'auto' }}>
            {delta > 0 ? "+" : ""}{delta}
          </span>
        )}
      </div>
    </div>
  );
}

window.ObsConsolidation = ObsConsolidation;
