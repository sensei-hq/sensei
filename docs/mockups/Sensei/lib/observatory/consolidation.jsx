// Memory consolidation review.
//
// Sensei has spotted overlapping memories. Each proposal shows the source
// memories and the merged result the system would create. The user
// accepts (sources archived → merged created → past_memories preserves
// the audit trail) or keeps them separate.

const { useState: cnS } = React;

function ObsConsolidation({ state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="統"
    emptyTitle="Nothing to consolidate"
    emptyHint="When memories start to overlap, sensei proposes merges here. Keep working — candidates form as evidence accumulates."
    errorHint="Couldn't load consolidation candidates. Try again." onRetry={() => {}} />;
  const items = window.UPGRADES.consolidations;
  const [openId, setOpen] = cnS(items[0].id);
  const [decisions, setDecisions] = cnS({});  // id → "merged" | "kept"
  const item = items.find(x => x.id === openId) || items[0];

  const decide = (id, choice) => setDecisions({ ...decisions, [id]: choice });

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Memory consolidation"
 >

      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>結</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Memories · consolidation
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            Three pairs of memories say nearly the same thing.
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            Merging keeps the canonical statement, combines evidence, and
            archives the originals. The audit trail is preserved in
            <span className="mono"> history.past_memories </span>so nothing
            is lost — only deduplicated.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <UgMini n={items.length} l="proposals"/>
          <UgMini n={items.reduce((s,x)=>s+x.sources.length,0)} l="memories"/>
          <UgMini n={`-${items.reduce((s,x)=>s+x.sources.length-1,0)}`}
                  l="net reduction" mono accent/>
        </div>
      </div>

      <div className="flex-1 grid min-h-0" style={{
 gridTemplateColumns: '320px 1fr' }}>
        {/* Proposal list */}
        <aside className="py-2 px-0 border-r overflow-auto" >
          {items.map(p => {
            const open = openId === p.id;
            const d = decisions[p.id];
            return (
              <button key={p.id} onClick={() => setOpen(p.id)}
 style={{
 background: open ? 'var(--paper-2)' : 'transparent',
 borderLeft: open ? '2px solid var(--accent)'
 : '2px solid transparent' }} className="py-3 px-4 w-full text-left cursor-pointer" >
                <div className="gap-2 mb-1 flex items-center" >
                  <span className="kanji text-accent" style={{ fontSize: 13 }}>結</span>
                  <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
                    {p.sources.length} → 1
                  </span>
                  <span className="flex-1" />
                  {d && (
                    <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.1em',
 color: d === "merged" ? 'var(--success)' : 'var(--ink-3)' }}>
                      {d}
                    </span>
                  )}
                </div>
                <div style={{
 fontSize: 13,
 color: open ? 'var(--ink)' : 'var(--ink-2)',
 lineHeight: 1.4 }} className="mb-1 font-medium" >
                  {p.title}
                </div>
                <div className="mono text-ink-4" style={{ fontSize: 11,
 lineHeight: 1.5 }}>
                  {p.sourceIds.join(" + ")}
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main className="pt-6 pb-8 px-8 overflow-auto" >
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
 fontSize: 11, letterSpacing: '0.12em' }} className="gap-3 mb-3 flex items-center text-ink-3 uppercase" >
        <span>Consolidation proposal</span>
        <Sep/>
        <span className="mono" style={{ letterSpacing: 0 }}>{p.id}</span>
      </div>
      <h2 className="display mt-0 mb-2 font-light text-ink" style={{
 fontSize: 28,
 lineHeight: 1.2, letterSpacing: '-0.015em' }}>
        {p.title}
      </h2>
      <p style={{
 fontSize: 13, lineHeight: 1.65, maxWidth: 720
 }} className="mt-0 mb-6 text-ink-2" >{p.reason}</p>

      {/* Sources column → Merged column visualization */}
      <div style={{ gridTemplateColumns: '1fr 24px 1fr' }} className="mb-6 gap-0 grid items-stretch" >

        {/* Sources */}
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-3 text-ink-3 uppercase" >
            Source memories ({p.sources.length})
          </div>
          <div className="gap-2 flex flex-col" >
            {p.sources.map(s => <MemorySnippet key={s.id} m={s} dim/>)}
          </div>
        </div>

        {/* Arrow column */}
        <div className="px-1 flex flex-col items-center justify-center" >
          <div className="flex-1" style={{ width: 1, background: 'var(--edge)' }}/>
          <span className="kanji my-2 mx-0 text-accent" style={{
 fontSize: 17 }}>→</span>
          <div className="flex-1" style={{ width: 1, background: 'var(--edge)' }}/>
        </div>

        {/* Proposed merged */}
        <div>
          <div className="mb-3 gap-1 flex items-center" >
            <span className="text-accent uppercase font-medium" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
              Proposed merged memory
            </span>
            <span className="kanji text-accent" style={{ fontSize: 13 }}>新</span>
          </div>
          <MergedMemory m={p.proposed}/>
        </div>
      </div>

      {/* Diff strip — what changes about evidence + strength */}
      <div style={{ gridTemplateColumns: 'repeat(4, 1fr)', background: 'var(--edge)',
 borderRadius: 6 }} className="gap-1 mb-6 grid overflow-hidden" >
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
        <div className="gap-2 pt-1 flex items-center" >
          <button onClick={() => onDecide("merged")}
 style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-ink text-paper border-0 cursor-pointer inline-flex items-center" >
            <span className="kanji text-accent" style={{ fontSize: 13 }}>結</span>
            Accept · merge & archive originals
          </button>
          <button onClick={() => onDecide("kept")}
 style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-paper-2 text-ink border border-paper-edge cursor-pointer inline-flex items-center" >
            <span className="kanji text-ink-3" style={{ fontSize: 13 }}>別</span>
            Keep separate
          </button>
          <span className="flex-1" />
          <FlatBtn glyph="編" label="Edit before merging"/>
        </div>
      ) : (
        <div style={{
 background: decision === "merged" ? 'var(--success-soft)'
 : 'var(--paper-2)',
 borderLeft: `2px solid ${decision === "merged" ? 'var(--success)' : 'var(--ink-3)'}`,
 borderRadius: 6 }} className="py-3 px-4 gap-3 border border-paper-edge flex items-center" >
          <span className="kanji" style={{ fontSize: 15,
                        color: decision === "merged" ? 'var(--success)' : 'var(--ink-3)' }}>
            {decision === "merged" ? "結" : "別"}
          </span>
          <div className="flex-1 text-ink" style={{ fontSize: 13 }}>
            {decision === "merged"
              ? <>Merged. <span className="mono text-ink-3" >
                    {p.sourceIds.join(", ")}
                  </span> archived to history.past_memories.</>
              : <>Kept separate. Sensei will not surface this proposal again
                  unless new evidence accumulates.</>}
          </div>
          <button onClick={() => onDecide(null)}
 style={{
 fontSize: 11,
 borderRadius: 4 }} className="py-1 px-2 text-ink-3 bg-transparent border border-paper-edge cursor-pointer" >undo</button>
        </div>
      )}
    </div>
  );
}

function MemorySnippet({ m, dim }) {
  return (
    <div style={{
 borderRadius: 5,
 opacity: dim ? 0.78 : 1
 }} className="py-3 px-3 bg-paper-2 border border-paper-edge" >
      <div className="gap-2 mb-1 flex items-baseline" >
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{m.id}</span>
        <span className="flex-1" />
        <span className="mono text-accent" style={{ fontSize: 11 }}>
          str {m.strength}
        </span>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {m.evidence.length} ev
        </span>
        {m.violated > 0 && (
          <span className="mono text-warning" style={{ fontSize: 11 }}>
            {m.violated}× violated
          </span>
        )}
      </div>
      <div style={{
 fontSize: 13, lineHeight: 1.5
 }} className="mb-1 text-ink" >{m.what}</div>
      <div className="text-ink-2 italic" style={{ fontSize: 11, lineHeight: 1.55 }}>{m.because}</div>
    </div>
  );
}

function MergedMemory({ m }) {
  return (
    <div style={{
 border: '1px solid var(--accent)',
 borderRadius: 5,
 boxShadow: '0 1px 0 var(--edge)'
 }} className="py-3 px-3 bg-paper" >
      <div className="gap-2 mb-1 flex items-baseline" >
        <span className="mono text-accent" style={{ fontSize: 11 }}>m-merged-pending</span>
        <span className="flex-1" />
        <span className="mono text-accent" style={{ fontSize: 11 }}>
          str {m.strength}
        </span>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {m.evidence.length} ev
        </span>
      </div>
      <div style={{
 fontSize: 13, lineHeight: 1.5 }} className="mb-1 text-ink font-medium" >{m.what}</div>
      <div style={{
 fontSize: 11, lineHeight: 1.55 }} className="mb-2 text-ink-2 italic" >{m.because}</div>
      <div className="gap-1 pt-2 flex flex-wrap border-t" >
        <ScopeChip s={m.scope}/>
        {m.scope.filePatterns?.map(g => (
          <span key={g} className="mono py-1 px-1 bg-paper-2 text-ink-3" style={{
 fontSize: 11,
 borderRadius: 3 }}>{g}</span>
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
    <span className="mono py-1 px-2 bg-paper-2 text-ink-2" style={{
 fontSize: 11,
 borderRadius: 3,
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
    <div className="py-3 px-3 bg-paper-2" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-4 uppercase" >{label}</div>
      <div className="gap-2 flex items-baseline" >
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{before}</span>
        <span className="text-ink-4" style={{ fontSize: 11 }}>→</span>
        <span className="display font-normal text-ink" style={{ fontSize: 17 }}>{after}</span>
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
