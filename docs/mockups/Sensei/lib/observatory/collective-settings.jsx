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

function ObsCollectiveSettings({ state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="群"
    emptyTitle="Sharing not set up yet"
    emptyHint="Choose what leaves your machine and how often. Connect a source to configure sharing mode, cadence and per-category filters."
    errorHint="Couldn't load your sharing settings. Try again." onRetry={() => {}} />;
  const U = window.UPGRADES;
  const [mode, setMode] = ciSt(U.sharingMode);
  const [cadence, setCadence] = ciSt(U.cadence);
  const [cats, setCats] = ciSt(SHARE_CATEGORIES);

  const toggleCat = (id) =>
    setCats(cats.map(c => c.id === id ? { ...c, enabled: !c.enabled } : c));

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Settings · Collective intelligence"
 >

      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>群</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Settings · Collective intelligence
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            What sensei shares with the network.
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            You agreed to share anonymized insights at setup. This is where
            you change how, what, and how often. Source code, prompts, file
            paths and project names never leave your machine.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <UgMini n={U.contribution.insightsShared} l="lifetime"/>
          <UgMini n={U.contribution.usersHelped} l="users helped" accent/>
          <UgMini n={U.contribution.streak} l="week streak" mono/>
        </div>
      </div>

      <div style={{
 maxWidth: 980 }} className="pt-8 pb-12 px-16 mx-auto flex-1 overflow-auto w-full" >

        {/* Mode picker */}
        <Section title="Sharing mode"
                 sub="Choose how anonymized insights leave your machine.">
          <div style={{ gridTemplateColumns: 'repeat(3, 1fr)'
 }} className="gap-2 grid" >
            {Object.entries(MODE_META).map(([k, m]) => {
              const on = mode === k;
              return (
                <button key={k} onClick={() => setMode(k)}
 style={{
 background: on ? 'var(--paper)' : 'var(--paper-2)',
 border: on ? '1px solid var(--accent)' : 'var(--hairline)',
 borderRadius: 6 }} className="py-4 px-4 text-left cursor-pointer" >
                  <div className="gap-2 mb-2 flex items-center" >
                    <span className="kanji" style={{ fontSize: 17,
                                  color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{m.glyph}</span>
                    <span className="display font-normal" style={{ fontSize: 15,
 color: on ? 'var(--ink)' : 'var(--ink-2)' }}>{m.label}</span>
                    <span className="flex-1" />
                    <span className="rounded-full" style={{ width: 14, height: 14,
 border: '1.5px solid',
 borderColor: on ? 'var(--accent)' : 'var(--ink-4)',
 background: on ? 'var(--accent)' : 'transparent' }}/>
                  </div>
                  <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.55 }}>
                    {m.blurb}
                  </div>
                </button>
              );
            })}
          </div>

          {mode !== "off" && (
            <div className="gap-2 mt-3 pt-3 flex items-center border-t" >
              <span className="text-ink-3 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
                Cadence
              </span>
              {["daily", "weekly", "monthly"].map(c => (
                <UgChip key={c} active={cadence === c} onClick={() => setCadence(c)}>
                  {c}
                </UgChip>
              ))}
              <span className="flex-1" />
              {mode === "review" && (
                <button className="text-accent bg-transparent border-0 cursor-pointer" style={{ fontSize: 11 }}>
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
          <div style={{ gridTemplateColumns: '1fr 1fr',
 background: 'var(--edge)',
 borderRadius: 6 }} className="gap-1 grid overflow-hidden" >
            {cats.map(c => (
              <label key={c.id}
 className="gap-3 py-3 px-4 flex items-start cursor-pointer bg-paper-2" >
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
                <div className="flex-1 min-w-0" >
                  <div style={{
 fontSize: 13 }} className="mb-1 text-ink font-medium" >{c.label}</div>
                  <div className="text-ink-2" style={{ fontSize: 11,
 lineHeight: 1.5 }}>{c.blurb}</div>
                </div>
              </label>
            ))}
          </div>
        </Section>

        {/* Sharing history */}
        <Section title="Sharing history"
                 sub="Every batch sensei has shipped on your behalf. Click any to see what was in it.">
          <div className="flex flex-col" >
            {U.sharingHistory.map(b => (
              <button key={b.id}
 style={{
 gridTemplateColumns: '90px 60px 1fr auto' }} className="gap-4 py-3 px-2 grid items-center text-left border-b bg-transparent cursor-pointer" >
                <span className="mono text-ink-2" style={{ fontSize: 11 }}>
                  {b.date}
                </span>
                <span className="display font-normal text-ink" style={{ fontSize: 15 }}>{b.insights}</span>
                <span className="gap-1 flex flex-wrap" >
                  {b.categories.map(cat => {
                    const m = SHARE_CATEGORIES.find(x => x.id === cat) ||
                              { glyph: "?", label: cat };
                    return (
                      <span key={cat} className="mono py-1 px-2 gap-1 bg-paper-2 text-ink-3 inline-flex items-center"
 style={{
 fontSize: 11,
 borderRadius: 3 }}>
                        <span className="kanji text-accent" style={{ fontSize: 11 }}>{m.glyph}</span>
                        {m.label.toLowerCase()}
                      </span>
                    );
                  })}
                </span>
                <span className="mono text-success" style={{ fontSize: 11 }}>
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
 borderRadius: 8, gridTemplateColumns: 'repeat(4, 1fr)'
 }} className="py-6 px-6 gap-6 bg-paper-2 border border-paper-edge grid" >
            <BigStat n={U.contribution.insightsShared} l="insights shared"/>
            <BigStat n={U.contribution.usersHelped} l="users helped" accent/>
            <BigStat n={U.contribution.streak} l="weekly streak" mono/>
            <BigStat n={U.contribution.rank} l="contributor rank" mono accent/>
          </div>
          <div style={{
 fontSize: 13, lineHeight: 1.6, borderRadius: 6 }} className="mt-4 py-3 px-4 gap-2 text-ink-2 bg-success-soft flex items-start" >
            <span className="kanji text-success" style={{ fontSize: 13 }}>礼</span>
            <span>
              Your <span className="font-medium" >{U.contribution.bestCategory}</span>{" "}
              insights have been your strongest contribution
              ({U.contribution.bestCategoryCount} shared).
              The {U.contribution.usersHelped} senseis who used them are anonymous to you,
              and you are anonymous to them.
            </span>
          </div>
        </Section>

        {/* Danger / privacy zone */}
        <Section title="Privacy controls" sub="">
          <div className="gap-1 flex flex-col" >
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
    <section style={{ opacity: dim ? 0.4 : 1 }} className="mb-8" >
      <div className="mb-3" >
        <h2 className="display m-0 font-medium text-ink" style={{
 fontSize: 15, letterSpacing: '-0.005em'
 }}>{title}</h2>
        {sub && (
          <p style={{
 fontSize: 13,
 lineHeight: 1.55, maxWidth: 720
 }} className="mt-1 mb-0 text-ink-2" >{sub}</p>
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
 fontSize: 11,
 letterSpacing: '0.12em' }} className="mt-1 text-ink-3 uppercase" >{l}</div>
    </div>
  );
}

function PrivacyRow({ glyph, label, sub, danger }) {
  return (
    <button style={{
 gridTemplateColumns: 'auto 1fr auto',
 borderRadius: 6 }} className="gap-3 py-3 px-4 grid items-center bg-paper-2 border border-paper-edge cursor-pointer text-left" >
      <span className="kanji" style={{ fontSize: 17,
                    color: danger ? 'var(--accent)' : 'var(--ink-3)' }}>{glyph}</span>
      <div>
        <div className="font-medium" style={{ fontSize: 13, color: danger ? 'var(--accent)' : 'var(--ink)' }}>{label}</div>
        <div style={{
 fontSize: 11, lineHeight: 1.5
 }} className="mt-1 text-ink-2" >{sub}</div>
      </div>
      <span className="text-ink-3" style={{ fontSize: 13 }}>→</span>
    </button>
  );
}

window.ObsCollectiveSettings = ObsCollectiveSettings;
