// Sharing review — what the *next* batch of contributions will include.
// Lives in the Memory section's settings flow: "Review before sharing".
//
// Header: scheduled date + cadence + count.
// Body: each insight as a card — category · title · summary · anonymization
//        note · evidence count · confidence. Each card has include/exclude.
// Footer: contribution-summary mini + "Send batch now" / "Hold this batch".

const { useState: shS } = React;

const CAT_META = {
  pattern:      { glyph: "紋", label: "Pattern",       color: "var(--shu)"   },
  anti_pattern: { glyph: "禁", label: "Anti-pattern",  color: "var(--amber)" },
  correction:   { glyph: "直", label: "Correction",    color: "var(--sumi-2)"},
  ftr:          { glyph: "果", label: "FTR signal",    color: "var(--jade)"  },
  model:        { glyph: "型", label: "Model",         color: "var(--sumi-2)"},
  skill:        { glyph: "技", label: "Skill",         color: "var(--jade)"  },
  tool:         { glyph: "具", label: "Tool",          color: "var(--sumi-2)"},
  stack:        { glyph: "層", label: "Stack",         color: "var(--shu)"   }
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
      <div style={{ padding: '22px 36px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 22 }}>
        <div className="kanji" style={{ fontSize: 42, color: 'var(--shu)', lineHeight: 1 }}>共</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 5 }}>
            Memories · review before sharing
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--sumi)' }}>
            The next share will include {included.length} insights.
          </h1>
          <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            Scheduled for <span style={{ color: 'var(--sumi)' }}>{U.nextBatch.scheduledFor}</span>{" "}
            ({U.cadence}). Sensei anonymizes paths, project names and identifiers
            before any item leaves your machine. Uncheck anything you'd rather keep private.
          </p>
        </div>
        <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 22 }}>
          <UgMini n={included.length} l="will share" accent/>
          <UgMini n={excluded.size} l="excluded"/>
          <UgMini n={U.contribution.streak} l="week streak" mono/>
        </div>
      </div>

      <div style={{ flex: 1, overflow: 'auto', minHeight: 0,
                     padding: '24px 36px 28px', maxWidth: 980, margin: '0 auto', width: '100%' }}>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 28 }}>
          {U.nextBatch.insights.map(ins => {
            const cm = CAT_META[ins.category] || CAT_META.pattern;
            const out = excluded.has(ins.id);
            return (
              <article key={ins.id}
                       style={{ display: 'grid',
                                 gridTemplateColumns: '24px 1fr auto',
                                 gap: 16, alignItems: 'flex-start',
                                 padding: '16px 18px',
                                 background: out ? 'transparent' : 'var(--paper-2)',
                                 border: 'var(--hairline)', borderRadius: 6,
                                 opacity: out ? 0.5 : 1 }}>
                <input type="checkbox" checked={!out}
                       onChange={() => toggle(ins.id)}
                       style={{ marginTop: 4, accentColor: 'var(--shu)',
                                 cursor: 'pointer', width: 14, height: 14 }}/>
                <div style={{ minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                    <span className="kanji" style={{ fontSize: 13, color: cm.color }}>{cm.glyph}</span>
                    <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                                    textTransform: 'uppercase' }}>{cm.label}</span>
                    <Sep/>
                    <span style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
                      {ins.evidence} evidence
                    </span>
                    <Sep/>
                    <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
                      conf {Math.round(ins.confidence*100)}%
                    </span>
                  </div>
                  <div style={{ fontSize: 14, color: 'var(--sumi)', lineHeight: 1.4,
                                 fontWeight: 500, marginBottom: 4 }}>{ins.title}</div>
                  <div style={{ fontSize: 12.5, color: 'var(--sumi-2)', lineHeight: 1.55,
                                 marginBottom: 8 }}>{ins.summary}</div>
                  <div style={{ display: 'flex', alignItems: 'flex-start', gap: 6,
                                 fontSize: 11, color: 'var(--sumi-3)', lineHeight: 1.5 }}>
                    <span className="kanji" style={{ fontSize: 11, color: 'var(--jade)',
                                  marginTop: 1 }}>匿</span>
                    <span style={{ fontStyle: 'italic' }}>{ins.anonymizationNote}</span>
                  </div>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4,
                               alignItems: 'flex-end' }}>
                  <button style={{ fontSize: 10.5, padding: '4px 9px',
                                    background: 'transparent',
                                    border: 'var(--hairline)', borderRadius: 4,
                                    color: 'var(--sumi-2)', cursor: 'pointer' }}>
                    view source →
                  </button>
                  <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>
                    {ins.sourceId}
                  </span>
                </div>
              </article>
            );
          })}
        </div>

        {/* Contribution summary */}
        <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 8, padding: '18px 22px', marginBottom: 22 }}>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 12 }}>
            Your contribution to the network
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 14 }}>
            <ContribStat n={U.contribution.insightsShared} l="insights shared"/>
            <ContribStat n={U.contribution.usersHelped} l="users helped" accent/>
            <ContribStat n={U.contribution.bestCategory} l={`best · ${U.contribution.bestCategoryCount}× · pattern`} mono/>
            <ContribStat n={U.contribution.rank} l="contributor rank" mono/>
          </div>
        </div>

        {/* Footer actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <button style={{ padding: '9px 18px', fontSize: 13,
                            background: 'var(--sumi)', color: 'var(--paper)',
                            border: 'none', borderRadius: 6, cursor: 'pointer',
                            display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>送</span>
            Send {included.length} insights now
          </button>
          <FlatBtn glyph="待" label="Hold this batch"/>
          <span style={{ flex: 1 }}/>
          <button style={{ fontSize: 11, color: 'var(--sumi-3)',
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
           style={{ fontSize: mono ? 17 : 22, color: accent ? 'var(--jade)' : 'var(--sumi)',
                     fontWeight: 300, lineHeight: 1.1 }}>
        {n}
      </div>
      <div style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 4,
                     letterSpacing: '0.1em', textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

window.ObsSharingReview = ObsSharingReview;
