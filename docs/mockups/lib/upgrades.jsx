// Upgrades — incoming gifts from the collective knowledge base.
//
// What other senseis have learned and packaged for you. Each item is a
// candidate; the user reviews and installs (or dismisses).
//
// Layout:
//   ▸ Hero — count + cadence summary
//   ▸ Filter bar — kind (agent · skill · command · lint) + project relevance
//   ▸ Two-pane: list of candidates (left) + detail (right)
//
// The detail pane shows the full anatomy: what · why for you · contributors
// + adoption · preview steps · conflicts with existing memories. Footer:
// install · defer · dismiss.

const { useState: ugS, useMemo: ugM } = React;

const KIND_META = {
  agent:   { glyph: "作", label: "Agent",   color: "var(--accent)"   },
  skill:   { glyph: "技", label: "Skill",   color: "var(--success)"  },
  command: { glyph: "令", label: "Command", color: "var(--warning)" },
  lint:    { glyph: "禁", label: "Lint",    color: "var(--ink-2)"}
};

function ObsUpgrades() {
  const U = window.UPGRADES;
  const [kindFilter, setKindFilter] = ugS("all");
  const [openId, setOpen] = ugS(U.incoming[0].id);

  const filtered = ugM(() => {
    if (kindFilter === "all") return U.incoming;
    return U.incoming.filter(u => u.kind === kindFilter);
  }, [kindFilter]);

  const item = filtered.find(x => x.id === openId) || filtered[0];

  return (
    <div className="sensei" data-screen-label="Observatory · Upgrades"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* Hero */}
      <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 24 }}>
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>贈</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            Observatory · Upgrades
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>
            Five gifts from the collective.
          </h1>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            Agents · skills · commands · lints — packaged from the network's
            shared insights. Each is matched to your stack and current memories.
          </p>
        </div>
        <div style={{ paddingLeft: 24, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 24 }}>
          <UgMini n={U.incoming.length} l="received"/>
          <UgMini n="weekly" l="cadence" mono/>
          <UgMini n={`+${Math.round(U.incoming.reduce((s,u)=>s+u.avgFtrLift,0)*100/U.incoming.length)}%`}
                  l="avg ftr lift" mono accent/>
        </div>
      </div>

      {/* Filter bar */}
      <div style={{ padding: '12px 32px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                        textTransform: 'uppercase' }}>kind</span>
        <UgChip active={kindFilter === "all"} onClick={() => setKindFilter("all")}>all</UgChip>
        {Object.entries(KIND_META).map(([k, m]) => (
          <UgChip key={k} active={kindFilter === k} onClick={() => setKindFilter(k)}
                  glyph={m.glyph}>{m.label}s</UgChip>
        ))}
        <span style={{ flex: 1 }}/>
        <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>
          {filtered.length} of {U.incoming.length}
        </span>
      </div>

      <div style={{ flex: 1, display: 'grid',
                     gridTemplateColumns: '320px 1fr',
                     minHeight: 0 }}>
        {/* List */}
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto', padding: '8px 0' }}>
          {filtered.map(u => {
            const km = KIND_META[u.kind];
            const open = openId === u.id;
            return (
              <button key={u.id} onClick={() => setOpen(u.id)}
                      style={{ width: '100%', textAlign: 'left',
                                padding: '12px 16px',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                 : '2px solid transparent',
                                cursor: 'pointer',
                                display: 'flex', flexDirection: 'column', gap: 4 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span className="kanji" style={{ fontSize: 13, color: km.color }}>{km.glyph}</span>
                  <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                                  textTransform: 'uppercase' }}>{km.label}</span>
                  <span style={{ flex: 1 }}/>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                    {u.received}
                  </span>
                </div>
                <div className="mono" style={{ fontSize: 11,
                              color: open ? 'var(--ink)' : 'var(--ink-2)',
                              lineHeight: 1.4 }}>
                  {u.name}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                               fontSize: 11, color: 'var(--ink-3)' }}>
                  <span>{u.contributors} sources</span>
                  <Sep/>
                  <span className="mono" style={{ color: 'var(--success)' }}>
                    +{Math.round(u.avgFtrLift*100)}% FTR
                  </span>
                  <Sep/>
                  <span style={{ color: u.maturity === "battle-tested" ? 'var(--success)' : 'var(--warning)' }}>
                    {u.maturity}
                  </span>
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main style={{ overflow: 'auto', padding: '32px 48px 32px' }}>
          {item && <UpgradeDetail item={item}/>}
        </main>
      </div>
    </div>
  );
}

function UpgradeDetail({ item }) {
  const km = KIND_META[item.kind];
  return (
    <div style={{ maxWidth: 720, margin: '0 auto' }}>
      {/* Eyebrow */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12,
                     fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase', marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 13, color: km.color, letterSpacing: 0 }}>{km.glyph}</span>
        <span>{km.label}</span>
        <Sep/>
        <span>{item.maturity}</span>
        <span style={{ flex: 1 }}/>
        <span style={{ color: 'var(--accent)' }}>{item.sourceModel}</span>
      </div>

      {/* Title */}
      <h2 className="display" style={{ fontSize: 28, fontWeight: 300, lineHeight: 1.2,
                                        letterSpacing: '-0.015em',
                                        margin: '0 0 8px', color: 'var(--ink)' }}>
        {item.title}
      </h2>
      <div className="mono" style={{ fontSize: 13, color: 'var(--ink-3)', marginBottom: 24 }}>
        {item.name}
      </div>

      {/* Summary */}
      <p style={{ fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.65,
                   margin: '0 0 24px' }}>{item.summary}</p>

      {/* Why for you */}
      <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderLeft: '2px solid var(--accent)',
                     borderRadius: 6, padding: '12px 16px', marginBottom: 24 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>Why for you</div>
        <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.6 }}>{item.why}</div>
      </div>

      {/* Stats grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                     gap: 0, borderTop: 'var(--hairline)',
                     borderBottom: 'var(--hairline)', marginBottom: 24 }}>
        <UgStat label="Contributors" value={item.contributors}/>
        <UgStat label="Adoptions"    value={item.adoptions} mono/>
        <UgStat label="Avg FTR lift" value={`+${Math.round(item.avgFtrLift*100)}%`} accent/>
        <UgStat label="Stack"        value={item.stack.join(" · ")} small/>
      </div>

      {/* Preview */}
      <div style={{ marginBottom: 24 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 8 }}>What it does</div>
        <ol style={{ margin: 0, padding: '0 0 0 16px',
                      display: 'flex', flexDirection: 'column', gap: 4 }}>
          {item.preview.map((p, i) => (
            <li key={i} style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.5 }}>
              {p}
            </li>
          ))}
        </ol>
      </div>

      {/* Conflicts */}
      {item.conflicts.length > 0 && (
        <div style={{ marginBottom: 24 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--warning)',
                         textTransform: 'uppercase', marginBottom: 8 }}>Touches existing memory</div>
          {item.conflicts.map((c, i) => (
            <div key={i} style={{ fontSize: 13, color: 'var(--ink-2)',
                                    background: 'var(--warning-soft)',
                                    borderRadius: 4, padding: '8px 12px', marginBottom: 4 }}>
              <span className="mono" style={{ color: 'var(--ink)' }}>{c.id}</span>
              {" — "}{c.note}
            </div>
          ))}
        </div>
      )}

      {/* Actions */}
      <div style={{ display: 'flex', gap: 8, alignItems: 'center', paddingTop: 16,
                     borderTop: 'var(--hairline)' }}>
        <button style={{ padding: '8px 16px', fontSize: 13,
                          background: 'var(--ink)', color: 'var(--paper)',
                          border: 'none', borderRadius: 6, cursor: 'pointer',
                          display: 'inline-flex', alignItems: 'center', gap: 8 }}>
          <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>受</span>
          Install for {item.relevantProjects.join(" · ")}
        </button>
        <FlatBtn glyph="試" label="Preview in sandbox"/>
        <FlatBtn glyph="後" label="Defer"/>
        <span style={{ flex: 1 }}/>
        <FlatBtn glyph="納" label="Dismiss" subtle/>
      </div>
    </div>
  );
}

function UgStat({ label, value, mono, accent, small }) {
  return (
    <div style={{ padding: '12px 0', borderRight: 'var(--hairline)' }}>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', marginBottom: 4 }}>{label}</div>
      <div className={mono ? "mono" : ""}
           style={{ fontSize: small ? 11.5 : 17, color: accent ? 'var(--success)' : 'var(--ink)',
                     lineHeight: 1.3, fontWeight: small ? 400 : 300 }}>
        {value}
      </div>
    </div>
  );
}

function UgMini({ n, l, accent, mono }) {
  return (
    <div style={{ textAlign: 'center' }}>
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 17, lineHeight: 1, fontWeight: 300,
                     color: accent ? 'var(--success)' : 'var(--ink)',
                     fontFeatureSettings: '"tnum"' }}>{n}</div>
      <div style={{ fontSize: 11, letterSpacing: '0.12em', color: 'var(--ink-4)',
                     marginTop: 4, textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

function UgChip({ active, onClick, glyph, children }) {
  return (
    <button onClick={onClick}
            style={{ padding: '4px 12px', fontSize: 11,
                      background: active ? 'var(--ink)' : 'transparent',
                      color: active ? 'var(--paper)' : 'var(--ink-2)',
                      border: active ? '1px solid var(--ink)' : '1px solid var(--edge)',
                      borderRadius: 20, cursor: 'pointer',
                      display: 'inline-flex', alignItems: 'center', gap: 4 }}>
      {glyph && (
        <span className="kanji" style={{ fontSize: 13,
                      color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{glyph}</span>
      )}
      {children}
    </button>
  );
}

function Sep() {
  return <span style={{ width: 3, height: 3, borderRadius: '50%',
                         background: 'var(--ink-4)', display: 'inline-block' }}/>;
}

window.ObsUpgrades = ObsUpgrades;
window.UgChip = UgChip;
window.UgMini = UgMini;
window.UgSep = Sep;
