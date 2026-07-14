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
    <div className="sensei" data-screen-label="Observatory · Memories · Next share"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* Hero */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>共</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Memories · review before sharing
          </div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            The next share will include {included.length} insights.
          </h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >
            Scheduled for <span style={{ color: 'var(--ink)' }}>{U.nextBatch.scheduledFor}</span>{" "}
            ({U.cadence}). Sensei anonymizes paths, project names and identifiers
            before any item leaves your machine. Uncheck anything you'd rather keep private.
          </p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex'
}} className="gap-5 pl-5" >
          <UgMini n={included.length} l="will share" accent/>
          <UgMini n={excluded.size} l="excluded"/>
          <UgMini n={U.contribution.streak} l="week streak" mono/>
        </div>
      </div>

      <div style={{
 flex: 1, overflow: 'auto', minHeight: 0, maxWidth: 980, width: '100%'
}} className="py-5 px-6 mx-auto" >

        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2 mb-5" >
          {U.nextBatch.insights.map(ins => {
            const cm = CAT_META[ins.category] || CAT_META.pattern;
            const out = excluded.has(ins.id);
            return (
              <article key={ins.id}
                       style={{
 display: 'grid',
                                 gridTemplateColumns: '24px 1fr auto', alignItems: 'flex-start',
                                 background: out ? 'transparent' : 'var(--paper-2)',
                                 border: 'var(--hairline)', borderRadius: 6,
                                 opacity: out ? 0.5 : 1
}} className="gap-4 py-4 px-4" >
                <input type="checkbox" checked={!out}
                       onChange={() => toggle(ins.id)}
                       style={{
 accentColor: 'var(--accent)',
                                 cursor: 'pointer', width: 14, height: 14
}} className="mt-1" />
                <div style={{ minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-1" >
                    <span className="kanji" style={{ fontSize: 13, color: cm.color }}>{cm.glyph}</span>
                    <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                                    textTransform: 'uppercase' }}>{cm.label}</span>
                    <Sep/>
                    <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                      {ins.evidence} evidence
                    </span>
                    <Sep/>
                    <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                      conf {Math.round(ins.confidence*100)}%
                    </span>
                  </div>
                  <div style={{
 fontSize: 13, color: 'var(--ink)', lineHeight: 1.4,
                                 fontWeight: 500
}} className="mb-1" >{ins.title}</div>
                  <div style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55
}} className="mb-2" >{ins.summary}</div>
                  <div style={{
 display: 'flex', alignItems: 'flex-start',
                                 fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.5
}} className="gap-1" >
                    <span className="kanji mt-1" style={{
 fontSize: 11, color: 'var(--success)'
}}>匿</span>
                    <span style={{ fontStyle: 'italic' }}>{ins.anonymizationNote}</span>
                  </div>
                </div>
                <div style={{
 display: 'flex', flexDirection: 'column',
                               alignItems: 'flex-end'
}} className="gap-1" >
                  <button style={{
 fontSize: 11,
                                    background: 'transparent',
                                    border: 'var(--hairline)', borderRadius: 4,
                                    color: 'var(--ink-2)', cursor: 'pointer'
}} className="py-1 px-2" >
                    view source →
                  </button>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                    {ins.sourceId}
                  </span>
                </div>
              </article>
            );
          })}
        </div>

        {/* Contribution summary */}
        <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 8
}} className="py-4 px-5 mb-5" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-3" >
            Your contribution to the network
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)' }} className="gap-3" >
            <ContribStat n={U.contribution.insightsShared} l="insights shared"/>
            <ContribStat n={U.contribution.usersHelped} l="users helped" accent/>
            <ContribStat n={U.contribution.bestCategory} l={`best · ${U.contribution.bestCategoryCount}× · pattern`} mono/>
            <ContribStat n={U.contribution.rank} l="contributor rank" mono/>
          </div>
        </div>

        {/* Footer actions */}
        <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
          <button style={{
 fontSize: 13,
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center'
}} className="py-2 px-4 gap-2" >
            <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>送</span>
            Send {included.length} insights now
          </button>
          <FlatBtn glyph="待" label="Hold this batch"/>
          <span style={{ flex: 1 }}/>
          <button style={{ fontSize: 11, color: 'var(--ink-3)',
                            background: 'transparent', border: 'none', cursor: 'pointer' }}>
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
 fontSize: 11, color: 'var(--ink-3)',
                     letterSpacing: '0.1em', textTransform: 'uppercase'
}} className="mt-1" >{l}</div>
    </div>
  );
}

window.ObsSharingReview = ObsSharingReview;
