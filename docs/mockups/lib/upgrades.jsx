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
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>贈</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Observatory · Upgrades
          </div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            Five gifts from the collective.
          </h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >
            Agents · skills · commands · lints — packaged from the network's
            shared insights. Each is matched to your stack and current memories.
          </p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex'
}} className="gap-5 pl-5" >
          <UgMini n={U.incoming.length} l="received"/>
          <UgMini n="weekly" l="cadence" mono/>
          <UgMini n={`+${Math.round(U.incoming.reduce((s,u)=>s+u.avgFtrLift,0)*100/U.incoming.length)}%`}
                  l="avg ftr lift" mono accent/>
        </div>
      </div>

      {/* Filter bar */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="py-3 px-6 gap-2" >
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
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto' }} className="py-2 px-0" >
          {filtered.map(u => {
            const km = KIND_META[u.kind];
            const open = openId === u.id;
            return (
              <button key={u.id} onClick={() => setOpen(u.id)}
                      style={{
 width: '100%', textAlign: 'left',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                 : '2px solid transparent',
                                cursor: 'pointer',
                                display: 'flex', flexDirection: 'column'
}} className="py-3 px-4 gap-1" >
                <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
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
                <div style={{
 display: 'flex', alignItems: 'center',
                               fontSize: 11, color: 'var(--ink-3)'
}} className="gap-2" >
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
        <main style={{ overflow: 'auto' }} className="py-6 px-7" >
          {item && <UpgradeDetail item={item}/>}
        </main>
      </div>
    </div>
  );
}

function UpgradeDetail({ item }) {
  const km = KIND_META[item.kind];
  return (
    <div style={{ maxWidth: 720 }} className="mx-auto" >
      {/* Eyebrow */}
      <div style={{
 display: 'flex', alignItems: 'center',
                     fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase'
}} className="gap-3 mb-3" >
        <span className="kanji" style={{ fontSize: 13, color: km.color, letterSpacing: 0 }}>{km.glyph}</span>
        <span>{km.label}</span>
        <Sep/>
        <span>{item.maturity}</span>
        <span style={{ flex: 1 }}/>
        <span style={{ color: 'var(--accent)' }}>{item.sourceModel}</span>
      </div>

      {/* Title */}
      <h2 className="display mt-0 mb-2" style={{
 fontSize: 28, fontWeight: 300, lineHeight: 1.2,
                                        letterSpacing: '-0.015em', color: 'var(--ink)'
}}>
        {item.title}
      </h2>
      <div className="mono mb-5" style={{ fontSize: 13, color: 'var(--ink-3)' }}>
        {item.name}
      </div>

      {/* Summary */}
      <p style={{
 fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.65
}} className="mt-0 mb-5" >{item.summary}</p>

      {/* Why for you */}
      <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderLeft: '2px solid var(--accent)',
                     borderRadius: 6
}} className="py-3 px-4 mb-5" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-1" >Why for you</div>
        <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.6 }}>{item.why}</div>
      </div>

      {/* Stats grid */}
      <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', borderTop: 'var(--hairline)',
                     borderBottom: 'var(--hairline)'
}} className="mb-5 gap-0" >
        <UgStat label="Contributors" value={item.contributors}/>
        <UgStat label="Adoptions"    value={item.adoptions} mono/>
        <UgStat label="Avg FTR lift" value={`+${Math.round(item.avgFtrLift*100)}%`} accent/>
        <UgStat label="Stack"        value={item.stack.join(" · ")} small/>
      </div>

      {/* Preview */}
      <div className="mb-5" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-2" >What it does</div>
        <ol style={{
                      display: 'flex', flexDirection: 'column'
}} className="gap-1 m-0 pl-4 pr-0" >
          {item.preview.map((p, i) => (
            <li key={i} style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.5 }}>
              {p}
            </li>
          ))}
        </ol>
      </div>

      {/* Conflicts */}
      {item.conflicts.length > 0 && (
        <div className="mb-5" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--warning)',
                         textTransform: 'uppercase'
}} className="mb-2" >Touches existing memory</div>
          {item.conflicts.map((c, i) => (
            <div key={i} style={{
 fontSize: 13, color: 'var(--ink-2)',
                                    background: 'var(--warning-soft)',
                                    borderRadius: 4
}} className="py-2 px-3 mb-1" >
              <span className="mono" style={{ color: 'var(--ink)' }}>{c.id}</span>
              {" — "}{c.note}
            </div>
          ))}
        </div>
      )}

      {/* Actions */}
      <div style={{
 display: 'flex', alignItems: 'center',
                     borderTop: 'var(--hairline)'
}} className="gap-2 pt-4" >
        <button style={{
 fontSize: 13,
                          background: 'var(--ink)', color: 'var(--paper)',
                          border: 'none', borderRadius: 6, cursor: 'pointer',
                          display: 'inline-flex', alignItems: 'center'
}} className="py-2 px-4 gap-2" >
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
    <div style={{ borderRight: 'var(--hairline)' }} className="py-3 px-0" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase'
}} className="mb-1" >{label}</div>
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
      <div style={{
 fontSize: 11, letterSpacing: '0.12em', color: 'var(--ink-4)', textTransform: 'uppercase'
}} className="mt-1" >{l}</div>
    </div>
  );
}

function UgChip({ active, onClick, glyph, children }) {
  return (
    <button onClick={onClick}
            style={{
 fontSize: 11,
                      background: active ? 'var(--ink)' : 'transparent',
                      color: active ? 'var(--paper)' : 'var(--ink-2)',
                      border: active ? '1px solid var(--ink)' : '1px solid var(--edge)',
                      borderRadius: 20, cursor: 'pointer',
                      display: 'inline-flex', alignItems: 'center'
}} className="py-1 px-3 gap-1" >
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
