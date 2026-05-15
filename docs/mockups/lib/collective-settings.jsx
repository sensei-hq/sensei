// Collective intelligence settings.
//
// Lives in the preferences area. The wizard promised this; this is where
// the user actually controls it after setup. Mode toggle + cadence +
// category filter + sharing history + lifetime contribution.

const { useState: ciSt } = React;

const SHARE_CATEGORIES = [
  { id: "pattern",      glyph: "紋", label: "Patterns",
    blurb: "Recurring shapes across projects",      enabled: true },
  { id: "anti_pattern", glyph: "禁", label: "Anti-patterns",
    blurb: "Things that consistently break",         enabled: true },
  { id: "correction",   glyph: "直", label: "Corrections",
    blurb: "Recurring rewrites you keep applying",   enabled: true },
  { id: "ftr",          glyph: "果", label: "FTR signals",
    blurb: "Aggregate first-try-right effects",      enabled: true },
  { id: "tool",         glyph: "具", label: "Tool usage",
    blurb: "Which MCP tools work for which stacks",  enabled: true },
  { id: "stack",        glyph: "層", label: "Stack stats",
    blurb: "Stack × tool × FTR aggregates",          enabled: true },
  { id: "model",        glyph: "型", label: "Model preferences",
    blurb: "Which models you reach for, never the prompts", enabled: false },
  { id: "skill",        glyph: "技", label: "Skills authored",
    blurb: "Skills you've written + their adoption", enabled: false }
];

const MODE_META = {
  auto:    { glyph: "送", label: "Auto-share",
             blurb: "Insights ship on cadence without prompting." },
  review:  { glyph: "閲", label: "Review before sharing",
             blurb: "You see each batch before it leaves your machine." },
  off:     { glyph: "封", label: "Off",
             blurb: "Nothing leaves the machine. You can still receive upgrades." }
};

function ObsCollectiveSettings() {
  const U = window.UPGRADES;
  const [mode, setMode] = ciSt(U.sharingMode);
  const [cadence, setCadence] = ciSt(U.cadence);
  const [cats, setCats] = ciSt(SHARE_CATEGORIES);

  const toggleCat = (id) =>
    setCats(cats.map(c => c.id === id ? { ...c, enabled: !c.enabled } : c));

  return (
    <div className="sensei" data-screen-label="Observatory · Settings · Collective intelligence"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* Hero */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>群</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Settings · Collective intelligence
          </div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            What sensei shares with the network.
          </h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >
            You agreed to share anonymized insights at setup. This is where
            you change how, what, and how often. Source code, prompts, file
            paths and project names never leave your machine.
          </p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex'
}} className="gap-5 pl-5" >
          <UgMini n={U.contribution.insightsShared} l="lifetime"/>
          <UgMini n={U.contribution.usersHelped} l="users helped" accent/>
          <UgMini n={U.contribution.streak} l="week streak" mono/>
        </div>
      </div>

      <div style={{
 flex: 1, overflow: 'auto',
                     maxWidth: 980, width: '100%'
}} className="pt-6 pb-7 px-8 mx-auto" >

        {/* Mode picker */}
        <Section title="Sharing mode"
                 sub="Choose how anonymized insights leave your machine.">
          <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)'
}} className="gap-2" >
            {Object.entries(MODE_META).map(([k, m]) => {
              const on = mode === k;
              return (
                <button key={k} onClick={() => setMode(k)}
                        style={{
 textAlign: 'left',
                                  background: on ? 'var(--paper)' : 'var(--paper-2)',
                                  border: on ? '1px solid var(--accent)' : 'var(--hairline)',
                                  borderRadius: 6, cursor: 'pointer'
}} className="py-4 px-4" >
                  <div style={{
 display: 'flex', alignItems: 'center'
}} className="gap-2 mb-2" >
                    <span className="kanji" style={{ fontSize: 17,
                                  color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{m.glyph}</span>
                    <span className="display" style={{ fontSize: 15, fontWeight: 400,
                                  color: on ? 'var(--ink)' : 'var(--ink-2)' }}>{m.label}</span>
                    <span style={{ flex: 1 }}/>
                    <span style={{ width: 14, height: 14, borderRadius: '50%',
                                    border: '1.5px solid',
                                    borderColor: on ? 'var(--accent)' : 'var(--ink-4)',
                                    background: on ? 'var(--accent)' : 'transparent' }}/>
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55 }}>
                    {m.blurb}
                  </div>
                </button>
              );
            })}
          </div>

          {mode !== "off" && (
            <div style={{
 display: 'flex', alignItems: 'center', borderTop: 'var(--hairline)'
}} className="gap-2 mt-3 pt-3" >
              <span style={{ fontSize: 11, color: 'var(--ink-3)',
                              letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                Cadence
              </span>
              {["daily", "weekly", "monthly"].map(c => (
                <UgChip key={c} active={cadence === c} onClick={() => setCadence(c)}>
                  {c}
                </UgChip>
              ))}
              <span style={{ flex: 1 }}/>
              {mode === "review" && (
                <button style={{ fontSize: 11, color: 'var(--accent)',
                                  background: 'transparent', border: 'none', cursor: 'pointer' }}>
                  Review next batch ({U.nextBatch.insights.length} insights) →
                </button>
              )}
            </div>
          )}
        </Section>

        {/* Category filter */}
        <Section title="What gets shared"
                 sub="Each category corresponds to one inference.insights type. Disable any you'd rather keep private."
                 dim={mode === "off"}>
          <div style={{
 display: 'grid', gridTemplateColumns: '1fr 1fr',
                         background: 'var(--edge)',
                         borderRadius: 6, overflow: 'hidden'
}} className="gap-1" >
            {cats.map(c => (
              <label key={c.id}
                     style={{
 display: 'flex', alignItems: 'flex-start', cursor: 'pointer',
                               background: 'var(--paper-2)'
}} className="gap-3 py-3 px-4" >
                <input type="checkbox" checked={c.enabled}
                       onChange={() => toggleCat(c.id)}
                       disabled={mode === "off"}
                       style={{
 accentColor: 'var(--accent)',
                                 width: 14, height: 14
}} className="mt-1" />
                <span className="kanji" style={{ fontSize: 15,
                              color: c.enabled ? 'var(--accent)' : 'var(--ink-4)',
                              marginTop: -1 }}>{c.glyph}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{
 fontSize: 13, color: 'var(--ink)',
                                 fontWeight: 500
}} className="mb-1" >{c.label}</div>
                  <div style={{ fontSize: 11, color: 'var(--ink-2)',
                                 lineHeight: 1.5 }}>{c.blurb}</div>
                </div>
              </label>
            ))}
          </div>
        </Section>

        {/* Sharing history */}
        <Section title="Sharing history"
                 sub="Every batch sensei has shipped on your behalf. Click any to see what was in it.">
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {U.sharingHistory.map(b => (
              <button key={b.id}
                      style={{
 display: 'grid',
                                gridTemplateColumns: '90px 60px 1fr auto', alignItems: 'center', textAlign: 'left',
                                borderBottom: 'var(--hairline)',
                                background: 'transparent', cursor: 'pointer'
}} className="gap-4 py-3 px-2" >
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>
                  {b.date}
                </span>
                <span className="display" style={{ fontSize: 15, fontWeight: 400,
                              color: 'var(--ink)' }}>{b.insights}</span>
                <span style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1" >
                  {b.categories.map(cat => {
                    const m = SHARE_CATEGORIES.find(x => x.id === cat) ||
                              { glyph: "?", label: cat };
                    return (
                      <span key={cat} className="mono py-1 px-2 gap-1"
                            style={{
 fontSize: 11,
                                      borderRadius: 3, background: 'var(--paper-2)',
                                      color: 'var(--ink-3)',
                                      display: 'inline-flex', alignItems: 'center'
}}>
                        <span className="kanji" style={{ fontSize: 11,
                                      color: 'var(--accent)' }}>{m.glyph}</span>
                        {m.label.toLowerCase()}
                      </span>
                    );
                  })}
                </span>
                <span className="mono" style={{ fontSize: 11, color: 'var(--success)' }}>
                  helped {b.helpedUsers}
                </span>
              </button>
            ))}
          </div>
        </Section>

        {/* Lifetime contribution */}
        <Section title="Lifetime contribution"
                 sub="Aggregate signal across every batch you've shipped.">
          <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                         borderRadius: 8,
                         display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)'
}} className="py-5 px-5 gap-5" >
            <BigStat n={U.contribution.insightsShared} l="insights shared"/>
            <BigStat n={U.contribution.usersHelped} l="users helped" accent/>
            <BigStat n={U.contribution.streak} l="weekly streak" mono/>
            <BigStat n={U.contribution.rank} l="contributor rank" mono accent/>
          </div>
          <div style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6,
                         background: 'var(--success-soft)', borderRadius: 6,
                         display: 'flex', alignItems: 'flex-start'
}} className="mt-4 py-3 px-4 gap-2" >
            <span className="kanji" style={{ fontSize: 13, color: 'var(--success)' }}>礼</span>
            <span>
              Your <span style={{ fontWeight: 500 }}>{U.contribution.bestCategory}</span>{" "}
              insights have been your strongest contribution
              ({U.contribution.bestCategoryCount} shared).
              The {U.contribution.usersHelped} senseis who used them are anonymous to you,
              and you are anonymous to them.
            </span>
          </div>
        </Section>

        {/* Danger / privacy zone */}
        <Section title="Privacy controls" sub="">
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            <PrivacyRow glyph="覗" label="Audit what's anonymized"
                        sub="Show the redaction transforms applied before any insight leaves the machine."/>
            <PrivacyRow glyph="戻" label="Recall a previous batch"
                        sub="Request the network unlink your contribution. May take up to 7 days to propagate."/>
            <PrivacyRow glyph="封" label="Disable & wipe contributions"
                        sub="Stop sharing and request all your prior contributions be unlinked."
                        danger/>
          </div>
        </Section>
      </div>
    </div>
  );
}

function Section({ title, sub, children, dim }) {
  return (
    <section style={{ opacity: dim ? 0.4 : 1 }} className="mb-6" >
      <div className="mb-3" >
        <h2 className="display m-0" style={{
 fontSize: 15, fontWeight: 500,
                      color: 'var(--ink)', letterSpacing: '-0.005em'
}}>{title}</h2>
        {sub && (
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       lineHeight: 1.55, maxWidth: 720
}} className="mt-1 mb-0" >{sub}</p>
        )}
      </div>
      {children}
    </section>
  );
}

function BigStat({ n, l, accent, mono }) {
  return (
    <div>
      <div className={mono ? "mono" : "display"}
           style={{ fontSize: mono ? 22 : 28, fontWeight: 300, lineHeight: 1.1,
                     color: accent ? 'var(--success)' : 'var(--ink)' }}>{n}</div>
      <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                     letterSpacing: '0.12em', textTransform: 'uppercase'
}} className="mt-1" >{l}</div>
    </div>
  );
}

function PrivacyRow({ glyph, label, sub, danger }) {
  return (
    <button style={{
 display: 'grid',
                      gridTemplateColumns: 'auto 1fr auto',
                      alignItems: 'center',
                      background: 'var(--paper-2)', border: 'var(--hairline)',
                      borderRadius: 6, cursor: 'pointer', textAlign: 'left'
}} className="gap-3 py-3 px-4" >
      <span className="kanji" style={{ fontSize: 17,
                    color: danger ? 'var(--accent)' : 'var(--ink-3)' }}>{glyph}</span>
      <div>
        <div style={{ fontSize: 13, color: danger ? 'var(--accent)' : 'var(--ink)',
                       fontWeight: 500 }}>{label}</div>
        <div style={{
 fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.5
}} className="mt-1" >{sub}</div>
      </div>
      <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>→</span>
    </button>
  );
}

window.ObsCollectiveSettings = ObsCollectiveSettings;
