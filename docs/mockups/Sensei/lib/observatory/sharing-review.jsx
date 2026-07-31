// Sharing review — what the *next* batch of contributions will include.
// Lives in the Memory section's settings flow: "Review before sharing".
//
// Header: scheduled date + cadence + count.
// Body: each insight as a card — category · title · summary · anonymization
//        note · evidence count · confidence. Each card has include/exclude.
// Footer: contribution-summary mini + "Send batch now" / "Hold this batch".

const { useState: shS } = React;

const CAT_META = {
  pattern:      { glyph: "紋", label: "Pattern",       color: "var(--accent)"   },
  anti_pattern: { glyph: "禁", label: "Anti-pattern",  color: "var(--warning)" },
  correction:   { glyph: "直", label: "Correction",    color: "var(--ink-2)"},
  ftr:          { glyph: "果", label: "FTR signal",    color: "var(--success)"  },
  model:        { glyph: "型", label: "Model",         color: "var(--ink-2)"},
  skill:        { glyph: "技", label: "Skill",         color: "var(--success)"  },
  tool:         { glyph: "具", label: "Tool",          color: "var(--ink-2)"},
  stack:        { glyph: "層", label: "Stack",         color: "var(--accent)"   }
};

function ObsSharingReview() {
  const U = window.UPGRADES;
  const [excluded, setExcluded] = shS(new Set());

  const toggle = (id) => {
    const next = new Set(excluded);
    if (next.has(id)) next.delete(id); else next.add(id);
    setExcluded(next);
  };

  const included = U.nextBatch.insights.filter(i => !excluded.has(i.id));

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Memories · Next share"
 >

      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>共</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Memories · review before sharing
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            The next share will include {included.length} insights.
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            Scheduled for <span className="text-ink" >{U.nextBatch.scheduledFor}</span>{" "}
            ({U.cadence}). Sensei anonymizes paths, project names and identifiers
            before any item leaves your machine. Uncheck anything you'd rather keep private.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <UgMini n={included.length} l="will share" accent/>
          <UgMini n={excluded.size} l="excluded"/>
          <UgMini n={U.contribution.streak} l="week streak" mono/>
        </div>
      </div>

      <div style={{ maxWidth: 980 }} className="py-6 px-8 mx-auto flex-1 overflow-auto min-h-0 w-full" >

        <div className="gap-2 mb-6 flex flex-col" >
          {U.nextBatch.insights.map(ins => {
            const cm = CAT_META[ins.category] || CAT_META.pattern;
            const out = excluded.has(ins.id);
            return (
              <article key={ins.id}
 style={{
 gridTemplateColumns: '24px 1fr auto',
 background: out ? 'transparent' : 'var(--paper-2)', borderRadius: 6,
 opacity: out ? 0.5 : 1
 }} className="gap-4 py-4 px-4 grid items-start border border-paper-edge" >
                <input type="checkbox" checked={!out}
 onChange={() => toggle(ins.id)}
 style={{
 accentColor: 'var(--accent)', width: 14, height: 14
 }} className="mt-1 cursor-pointer" />
                <div className="min-w-0" >
                  <div className="gap-2 mb-1 flex items-center" >
                    <span className="kanji" style={{ fontSize: 13, color: cm.color }}>{cm.glyph}</span>
                    <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{cm.label}</span>
                    <Sep/>
                    <span className="text-ink-3" style={{ fontSize: 11 }}>
                      {ins.evidence} evidence
                    </span>
                    <Sep/>
                    <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                      conf {Math.round(ins.confidence*100)}%
                    </span>
                  </div>
                  <div style={{
 fontSize: 13, lineHeight: 1.4 }} className="mb-1 text-ink font-medium" >{ins.title}</div>
                  <div style={{
 fontSize: 13, lineHeight: 1.55
 }} className="mb-2 text-ink-2" >{ins.summary}</div>
                  <div style={{
 fontSize: 11, lineHeight: 1.5
 }} className="gap-1 flex items-start text-ink-3" >
                    <span className="kanji mt-1 text-success" style={{
 fontSize: 11 }}>匿</span>
                    <span className="italic" >{ins.anonymizationNote}</span>
                  </div>
                </div>
                <div className="gap-1 flex flex-col items-end" >
                  <button style={{
 fontSize: 11, borderRadius: 4 }} className="py-1 px-2 bg-transparent border border-paper-edge text-ink-2 cursor-pointer" >
                    view source →
                  </button>
                  <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                    {ins.sourceId}
                  </span>
                </div>
              </article>
            );
          })}
        </div>

        {/* Contribution summary */}
        <div style={{
 borderRadius: 8
 }} className="py-4 px-6 mb-6 bg-paper-2 border border-paper-edge" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-3 text-ink-3 uppercase" >
            Your contribution to the network
          </div>
          <div style={{ gridTemplateColumns: 'repeat(4, 1fr)' }} className="gap-3 grid" >
            <ContribStat n={U.contribution.insightsShared} l="insights shared"/>
            <ContribStat n={U.contribution.usersHelped} l="users helped" accent/>
            <ContribStat n={U.contribution.bestCategory} l={`best · ${U.contribution.bestCategoryCount}× · pattern`} mono/>
            <ContribStat n={U.contribution.rank} l="contributor rank" mono/>
          </div>
        </div>

        {/* Footer actions */}
        <div className="gap-2 flex items-center" >
          <button style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-ink text-paper border-0 cursor-pointer inline-flex items-center" >
            <span className="kanji text-accent" style={{ fontSize: 13 }}>送</span>
            Send {included.length} insights now
          </button>
          <FlatBtn glyph="待" label="Hold this batch"/>
          <span className="flex-1" />
          <button className="text-ink-3 bg-transparent border-0 cursor-pointer" style={{ fontSize: 11 }}>
            sharing settings →
          </button>
        </div>
      </div>
    </div>
  );
}

function ContribStat({ n, l, accent, mono }) {
  return (
    <div>
      <div className={mono ? "mono" : "display"}
           style={{ fontSize: mono ? 17 : 22, color: accent ? 'var(--success)' : 'var(--ink)',
                     fontWeight: 300, lineHeight: 1.1 }}>
        {n}
      </div>
      <div style={{
 fontSize: 11,
 letterSpacing: '0.1em' }} className="mt-1 text-ink-3 uppercase" >{l}</div>
    </div>
  );
}

window.ObsSharingReview = ObsSharingReview;
