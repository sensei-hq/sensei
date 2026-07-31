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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Upgrades"
 >

      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>贈</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Observatory · Upgrades
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            Five gifts from the collective.
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            Agents · skills · commands · lints — packaged from the network's
            shared insights. Each is matched to your stack and current memories.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <UgMini n={U.incoming.length} l="received"/>
          <UgMini n="weekly" l="cadence" mono/>
          <UgMini n={`+${Math.round(U.incoming.reduce((s,u)=>s+u.avgFtrLift,0)*100/U.incoming.length)}%`}
                  l="avg ftr lift" mono accent/>
        </div>
      </div>

      {/* Filter bar */}
      <div className="py-3 px-8 gap-2 border-b flex items-center" >
        <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>kind</span>
        <UgChip active={kindFilter === "all"} onClick={() => setKindFilter("all")}>all</UgChip>
        {Object.entries(KIND_META).map(([k, m]) => (
          <UgChip key={k} active={kindFilter === k} onClick={() => setKindFilter(k)}
                  glyph={m.glyph}>{m.label}s</UgChip>
        ))}
        <span className="flex-1" />
        <span className="text-ink-4" style={{ fontSize: 11 }}>
          {filtered.length} of {U.incoming.length}
        </span>
      </div>

      <div className="flex-1 grid min-h-0" style={{
 gridTemplateColumns: '320px 1fr' }}>
        {/* List */}
        <aside className="py-2 px-0 border-r overflow-auto" >
          {filtered.map(u => {
            const km = KIND_META[u.kind];
            const open = openId === u.id;
            return (
              <button key={u.id} onClick={() => setOpen(u.id)}
 style={{
 background: open ? 'var(--paper-2)' : 'transparent',
 borderLeft: open ? '2px solid var(--accent)'
 : '2px solid transparent' }} className="py-3 px-4 gap-1 w-full text-left cursor-pointer flex flex-col" >
                <div className="gap-2 flex items-center" >
                  <span className="kanji" style={{ fontSize: 13, color: km.color }}>{km.glyph}</span>
                  <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{km.label}</span>
                  <span className="flex-1" />
                  <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                    {u.received}
                  </span>
                </div>
                <div className="mono" style={{ fontSize: 11,
                              color: open ? 'var(--ink)' : 'var(--ink-2)',
                              lineHeight: 1.4 }}>
                  {u.name}
                </div>
                <div style={{
 fontSize: 11 }} className="gap-2 flex items-center text-ink-3" >
                  <span>{u.contributors} sources</span>
                  <Sep/>
                  <span className="mono text-success" >
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
        <main className="py-8 px-12 overflow-auto" >
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
 fontSize: 11, letterSpacing: '0.12em' }} className="gap-3 mb-3 flex items-center text-ink-3 uppercase" >
        <span className="kanji" style={{ fontSize: 13, color: km.color, letterSpacing: 0 }}>{km.glyph}</span>
        <span>{km.label}</span>
        <Sep/>
        <span>{item.maturity}</span>
        <span className="flex-1" />
        <span className="text-accent" >{item.sourceModel}</span>
      </div>

      {/* Title */}
      <h2 className="display mt-0 mb-2 font-light text-ink" style={{
 fontSize: 28, lineHeight: 1.2,
 letterSpacing: '-0.015em' }}>
        {item.title}
      </h2>
      <div className="mono mb-6 text-ink-3" style={{ fontSize: 13 }}>
        {item.name}
      </div>

      {/* Summary */}
      <p style={{
 fontSize: 15, lineHeight: 1.65
 }} className="mt-0 mb-6 text-ink-2" >{item.summary}</p>

      {/* Why for you */}
      <div style={{
 borderLeft: '2px solid var(--accent)',
 borderRadius: 6
 }} className="py-3 px-4 mb-6 bg-paper-2 border border-paper-edge" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >Why for you</div>
        <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.6 }}>{item.why}</div>
      </div>

      {/* Stats grid */}
      <div style={{ gridTemplateColumns: 'repeat(4, 1fr)' }} className="mb-6 gap-0 grid border-t border-b" >
        <UgStat label="Contributors" value={item.contributors}/>
        <UgStat label="Adoptions"    value={item.adoptions} mono/>
        <UgStat label="Avg FTR lift" value={`+${Math.round(item.avgFtrLift*100)}%`} accent/>
        <UgStat label="Stack"        value={item.stack.join(" · ")} small/>
      </div>

      {/* Preview */}
      <div className="mb-6" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >What it does</div>
        <ol className="gap-1 m-0 pl-4 pr-0 flex flex-col" >
          {item.preview.map((p, i) => (
            <li className="text-ink-2" key={i} style={{ fontSize: 13, lineHeight: 1.5 }}>
              {p}
            </li>
          ))}
        </ol>
      </div>

      {/* Conflicts */}
      {item.conflicts.length > 0 && (
        <div className="mb-6" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-warning uppercase" >Touches existing memory</div>
          {item.conflicts.map((c, i) => (
            <div key={i} style={{
 fontSize: 13,
 borderRadius: 4
 }} className="py-2 px-3 mb-1 text-ink-2 bg-warning-soft" >
              <span className="mono text-ink" >{c.id}</span>
              {" — "}{c.note}
            </div>
          ))}
        </div>
      )}

      {/* Actions */}
      <div className="gap-2 pt-4 flex items-center border-t" >
        <button style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 gap-2 bg-ink text-paper border-0 cursor-pointer inline-flex items-center" >
          <span className="kanji text-accent" style={{ fontSize: 13 }}>受</span>
          Install for {item.relevantProjects.join(" · ")}
        </button>
        <FlatBtn glyph="試" label="Preview in sandbox"/>
        <FlatBtn glyph="後" label="Defer"/>
        <span className="flex-1" />
        <FlatBtn glyph="納" label="Dismiss" subtle/>
      </div>
    </div>
  );
}

function UgStat({ label, value, mono, accent, small }) {
  return (
    <div className="py-3 px-0 border-r" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-4 uppercase" >{label}</div>
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
    <div className="text-center" >
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 17, lineHeight: 1, fontWeight: 300,
                     color: accent ? 'var(--success)' : 'var(--ink)',
                     fontFeatureSettings: '"tnum"' }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mt-1 text-ink-4 uppercase" >{l}</div>
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
 borderRadius: 20 }} className="py-1 px-3 gap-1 cursor-pointer inline-flex items-center" >
      {glyph && (
        <span className="kanji" style={{ fontSize: 13,
                      color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{glyph}</span>
      )}
      {children}
    </button>
  );
}

function Sep() {
  return <span className="rounded-full bg-ink-4 inline-block" style={{ width: 3, height: 3 }}/>;
}

window.ObsUpgrades = ObsUpgrades;
window.UgChip = UgChip;
window.UgMini = UgMini;
window.UgSep = Sep;
