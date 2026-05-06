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
      <div style={{ padding: '22px 36px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 22 }}>
        <div className="kanji" style={{ fontSize: 42, color: 'var(--shu)', lineHeight: 1 }}>群</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 5 }}>
            Settings · Collective intelligence
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--sumi)' }}>
            What sensei shares with the network.
          </h1>
          <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            You agreed to share anonymized insights at setup. This is where
            you change how, what, and how often. Source code, prompts, file
            paths and project names never leave your machine.
          </p>
        </div>
        <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 22 }}>
          <UgMini n={U.contribution.insightsShared} l="lifetime"/>
          <UgMini n={U.contribution.usersHelped} l="users helped" accent/>
          <UgMini n={U.contribution.streak} l="week streak" mono/>
        </div>
      </div>

      <div style={{ flex: 1, overflow: 'auto', padding: '32px 60px 48px',
                     maxWidth: 980, margin: '0 auto', width: '100%' }}>

        {/* Mode picker */}
        <Section title="Sharing mode"
                 sub="Choose how anonymized insights leave your machine.">
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)',
                         gap: 10 }}>
            {Object.entries(MODE_META).map(([k, m]) => {
              const on = mode === k;
              return (
                <button key={k} onClick={() => setMode(k)}
                        style={{ textAlign: 'left',
                                  padding: '16px 18px',
                                  background: on ? 'var(--paper)' : 'var(--paper-2)',
                                  border: on ? '1px solid var(--shu)' : 'var(--hairline)',
                                  borderRadius: 6, cursor: 'pointer' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10,
                                 marginBottom: 8 }}>
                    <span className="kanji" style={{ fontSize: 18,
                                  color: on ? 'var(--shu)' : 'var(--sumi-3)' }}>{m.glyph}</span>
                    <span className="display" style={{ fontSize: 15, fontWeight: 400,
                                  color: on ? 'var(--sumi)' : 'var(--sumi-2)' }}>{m.label}</span>
                    <span style={{ flex: 1 }}/>
                    <span style={{ width: 14, height: 14, borderRadius: '50%',
                                    border: '1.5px solid',
                                    borderColor: on ? 'var(--shu)' : 'var(--sumi-4)',
                                    background: on ? 'var(--shu)' : 'transparent' }}/>
                  </div>
                  <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
                    {m.blurb}
                  </div>
                </button>
              );
            })}
          </div>

          {mode !== "off" && (
            <div style={{ display: 'flex', alignItems: 'center', gap: 10,
                           marginTop: 14, paddingTop: 14, borderTop: 'var(--hairline)' }}>
              <span style={{ fontSize: 11, color: 'var(--sumi-3)',
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
                <button style={{ fontSize: 11.5, color: 'var(--shu)',
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
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 1,
                         background: 'var(--paper-edge)',
                         borderRadius: 6, overflow: 'hidden' }}>
            {cats.map(c => (
              <label key={c.id}
                     style={{ display: 'flex', alignItems: 'flex-start', gap: 12,
                               padding: '14px 16px', cursor: 'pointer',
                               background: 'var(--paper-2)' }}>
                <input type="checkbox" checked={c.enabled}
                       onChange={() => toggleCat(c.id)}
                       disabled={mode === "off"}
                       style={{ marginTop: 2, accentColor: 'var(--shu)',
                                 width: 14, height: 14 }}/>
                <span className="kanji" style={{ fontSize: 16,
                              color: c.enabled ? 'var(--shu)' : 'var(--sumi-4)',
                              marginTop: -1 }}>{c.glyph}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: 'var(--sumi)',
                                 fontWeight: 500, marginBottom: 2 }}>{c.label}</div>
                  <div style={{ fontSize: 11.5, color: 'var(--sumi-2)',
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
                      style={{ display: 'grid',
                                gridTemplateColumns: '90px 60px 1fr auto',
                                gap: 16, alignItems: 'center',
                                padding: '12px 8px', textAlign: 'left',
                                borderBottom: 'var(--hairline)',
                                background: 'transparent', cursor: 'pointer' }}>
                <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi-2)' }}>
                  {b.date}
                </span>
                <span className="display" style={{ fontSize: 16, fontWeight: 400,
                              color: 'var(--sumi)' }}>{b.insights}</span>
                <span style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
                  {b.categories.map(cat => {
                    const m = SHARE_CATEGORIES.find(x => x.id === cat) ||
                              { glyph: "?", label: cat };
                    return (
                      <span key={cat} className="mono"
                            style={{ fontSize: 10, padding: '2px 7px',
                                      borderRadius: 3, background: 'var(--paper-2)',
                                      color: 'var(--sumi-3)',
                                      display: 'inline-flex', alignItems: 'center', gap: 4 }}>
                        <span className="kanji" style={{ fontSize: 11,
                                      color: 'var(--shu)' }}>{m.glyph}</span>
                        {m.label.toLowerCase()}
                      </span>
                    );
                  })}
                </span>
                <span className="mono" style={{ fontSize: 11, color: 'var(--jade)' }}>
                  helped {b.helpedUsers}
                </span>
              </button>
            ))}
          </div>
        </Section>

        {/* Lifetime contribution */}
        <Section title="Lifetime contribution"
                 sub="Aggregate signal across every batch you've shipped.">
          <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                         borderRadius: 8, padding: '22px 26px',
                         display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                         gap: 24 }}>
            <BigStat n={U.contribution.insightsShared} l="insights shared"/>
            <BigStat n={U.contribution.usersHelped} l="users helped" accent/>
            <BigStat n={U.contribution.streak} l="weekly streak" mono/>
            <BigStat n={U.contribution.rank} l="contributor rank" mono accent/>
          </div>
          <div style={{ fontSize: 12, color: 'var(--sumi-2)', lineHeight: 1.6,
                         marginTop: 16, padding: '12px 16px',
                         background: 'var(--jade-soft)', borderRadius: 6,
                         display: 'flex', gap: 10, alignItems: 'flex-start' }}>
            <span className="kanji" style={{ fontSize: 14, color: 'var(--jade)' }}>礼</span>
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
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
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
    <section style={{ marginBottom: 38, opacity: dim ? 0.4 : 1 }}>
      <div style={{ marginBottom: 14 }}>
        <h2 className="display" style={{ fontSize: 16, fontWeight: 500, margin: 0,
                      color: 'var(--sumi)', letterSpacing: '-0.005em' }}>{title}</h2>
        {sub && (
          <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                       lineHeight: 1.55, maxWidth: 720 }}>{sub}</p>
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
                     color: accent ? 'var(--jade)' : 'var(--sumi)' }}>{n}</div>
      <div style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 6,
                     letterSpacing: '0.12em', textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

function PrivacyRow({ glyph, label, sub, danger }) {
  return (
    <button style={{ display: 'grid',
                      gridTemplateColumns: 'auto 1fr auto', gap: 14,
                      alignItems: 'center', padding: '12px 16px',
                      background: 'var(--paper-2)', border: 'var(--hairline)',
                      borderRadius: 6, cursor: 'pointer', textAlign: 'left' }}>
      <span className="kanji" style={{ fontSize: 18,
                    color: danger ? 'var(--shu)' : 'var(--sumi-3)' }}>{glyph}</span>
      <div>
        <div style={{ fontSize: 12.5, color: danger ? 'var(--shu)' : 'var(--sumi)',
                       fontWeight: 500 }}>{label}</div>
        <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.5,
                       marginTop: 2 }}>{sub}</div>
      </div>
      <span style={{ fontSize: 12, color: 'var(--sumi-3)' }}>→</span>
    </button>
  );
}

window.ObsCollectiveSettings = ObsCollectiveSettings;
