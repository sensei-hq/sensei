// Share hub — the UNIFIED share + upgrades surface.
//
// Replaces the split global-only screens (ObsSharingReview / ObsUpgrades) and
// the dojo-only touchpoints (InappShare / InappDownstream) with ONE of each.
// The channel is an attribute of the item, not a separate screen — and a user
// belongs to MANY sources, so provenance is a NAMED source (which Dōjō, or the
// public collective) plus a SCOPE (which projects it applies to, or all):
//
//   ObsShareReviewHub — review-before-sharing with a DESTINATION selector
//        (the global collective or any one of your Dōjōs). Per-destination
//        framing keeps each trust boundary explicit (public/anonymized vs
//        org-internal/attributed vs client/dereferenced).
//   ObsUpgradesInbox  — one inbox with a SOURCE filter + named provenance
//        badges + scope. Global items carry adoption stats; Dōjō items carry
//        which Dōjō approved it and who.
//
// Self-contained; reads window.UPGRADES. Token-only → theme-free.

const { useState: shbS, useMemo: shbM } = React;

// ─── The user's sources (one global commons + many Dōjōs) ───
// kind drives color + the trust/attribution policy; label is the name shown.
const KIND = {
  global:    { color: "var(--accent)",  policy: "public · anonymized · opt-in" },
  employer:  { color: "var(--success)", policy: "org-internal · attributed · governed" },
  client:    { color: "var(--warning)", policy: "confidential · source-dereferenced" },
  community: { color: "var(--ink-2)",   policy: "community · opt-in · attributed" },
  personal:  { color: "var(--ink-3)",   policy: "private to you" },
};
const SOURCES = {
  global: { kanji: "環", label: "Global collective", kind: "global" },
  acme:   { kanji: "社", label: "Acme Corp",         kind: "employer" },
  lumen:  { kanji: "客", label: "Lumen (client)",    kind: "client" },
  guild:  { kanji: "群", label: "Rust Guild",        kind: "community" },
};
const srcColor = id => KIND[SOURCES[id].kind].color;

// per-upgrade provenance: which source it came from + scope (projects | "all")
// + governance detail for Dōjō sources.
const UPG_SOURCE = {
  "u-agent-rust-axum-handler": { src: "global" },
  "u-skill-retry-backoff":     { src: "global" },
  "u-command-check-handlers":  { src: "acme",  approver: "priya.n", team: "Platform", approvedOn: "2 days ago" },
  "u-lint-no-mock-db":         { src: "acme",  approver: "marco.s", team: "Payments", approvedOn: "1 week ago" },
  "u-skill-svelte5-state":     { src: "guild", approver: "koto-maintainers", approvedOn: "3 days ago" },
};
const srcOf = u => (UPG_SOURCE[u.id] || { src: "global" }).src;
const scopeLabel = u => {
  const p = u.relevantProjects || [];
  return p.length >= 3 ? "all projects" : p.join(" · ");
};

function ShbSep() {
  return <span className="rounded-full bg-ink-4 inline-block" style={{ width: 3, height: 3 }}/>;
}
function ShbMini({ n, l, accent, mono }) {
  return (
    <div className="text-center" >
      <div className={mono ? "mono" : ""} style={{ fontSize: 17, lineHeight: 1, fontWeight: 300,
            color: accent ? "var(--success)" : "var(--ink)", fontFeatureSettings: '"tnum"' }}>{n}</div>
      <div className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: "0.12em", marginTop: 4 }}>{l}</div>
    </div>
  );
}
function ShbFlat({ glyph, label, subtle, onClick }) {
  return (
    <button className="bg-transparent border-0 cursor-pointer inline-flex items-center" onClick={onClick} style={{ fontSize: 13,
 color: subtle ? "var(--ink-4)" : "var(--ink-2)", gap: 6, padding: "8px 12px" }}>
      {glyph && <span className="kanji text-ink-3" style={{ fontSize: 13 }}>{glyph}</span>}{label}
    </button>
  );
}
function ShbSeg({ options, value, onChange }) {
  return (
    <div className="inline-flex bg-paper-3 flex-wrap" style={{ borderRadius: 8, padding: 3, gap: 2 }}>
      {options.map(o => {
        const on = value === o.id;
        return (
          <button className="border-0 cursor-pointer inline-flex items-center" key={o.id} onClick={() => onChange(o.id)} style={{ borderRadius: 6, padding: "8px 12px",
 fontSize: 12.5, fontWeight: on ? 600 : 400, gap: 6,
 background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-3)",
 boxShadow: on ? "var(--shadow-sm)" : "none" }}>
            {o.kanji && <span className="kanji" style={{ fontSize: 14, color: on ? (o.color || "var(--accent)") : "var(--ink-4)" }}>{o.kanji}</span>}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
// named source pill (kanji + label, colored by kind)
function SrcBadge({ src, small }) {
  const s = SOURCES[src]; const c = srcColor(src);
  return (
    <span className="inline-flex items-center whitespace-nowrap" style={{ gap: 5,
 fontSize: small ? 10.5 : 11.5, letterSpacing: ".02em", color: c,
 background: `color-mix(in oklch, ${c} 12%, transparent)`,
 border: `1px solid color-mix(in oklch, ${c} 30%, transparent)`,
 borderRadius: 999, padding: small ? "2px 8px" : "3px 10px" }}>
      <span className="kanji" style={{ fontSize: small ? 11 : 12 }}>{s.kanji}</span>{s.label}
    </span>
  );
}
// scope pill — which projects the item applies to
function ScopePill({ label }) {
  return (
    <span className="inline-flex items-center text-ink-3 bg-paper-3 border border-paper-edge whitespace-nowrap" style={{ gap: 5, fontSize: 10.5, borderRadius: 999, padding: "4px 8px" }}>
      <span className="kanji text-ink-4" style={{ fontSize: 11 }}>及</span>{label}
    </span>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  SHARE REVIEW — unified, destination-aware
// ═══════════════════════════════════════════════════════════════════
const CAT_META = {
  pattern:      { glyph: "紋", label: "Pattern",      color: "var(--accent)" },
  anti_pattern: { glyph: "禁", label: "Anti-pattern", color: "var(--warning)" },
  correction:   { glyph: "直", label: "Correction",   color: "var(--ink-2)" },
  ftr:          { glyph: "果", label: "FTR signal",   color: "var(--success)" },
};

// destinations a share can target — the global commons + each Dōjō you're in
const SHARE_DESTS = ["global", "acme", "lumen", "guild"];

function ObsShareReviewHub() {
  const U = window.UPGRADES;
  const [dest, setDest] = shbS("acme");
  const [excluded, setExcluded] = shbS(new Set());
  const toggle = id => { const n = new Set(excluded); n.has(id) ? n.delete(id) : n.add(id); setExcluded(n); };
  const included = U.nextBatch.insights.filter(i => !excluded.has(i.id));
  const kind = SOURCES[dest].kind;
  const c = srcColor(dest);

  // per-kind framing keeps the trust boundary explicit
  const F = {
    global:    { verb: "Send",  em: "anonymized",         emEnd: " — paths, project names and identifiers are stripped before anything leaves your machine.", glyph: "送" },
    employer:  { verb: "Share", em: "attributed to you",  emEnd: " — teammates see who taught it, and it passes your org's review gates.", glyph: "受" },
    client:    { verb: "Share", em: "source-dereferenced", emEnd: " — the lesson, its cause and context travel, but the repo/identifiers are dropped so nothing leaks.", glyph: "秘" },
    community: { verb: "Share", em: "attributed to you",  emEnd: " — visible to guild members who opted in; nothing employer- or client-scoped is included.", glyph: "受" },
  }[kind];

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Share review"
 >
      {/* Hero */}
      <div className="pt-6 pb-4 px-8 border-b">
        <div className="gap-6 flex items-center">
          <div className="kanji" style={{ fontSize: 40, color: c, lineHeight: 1 }}>{SOURCES[dest].kanji}</div>
          <div className="flex-1 min-w-0" >
            <div style={{ fontSize: 11, letterSpacing: "0.18em" }} className="mb-1 text-ink-3 uppercase">Memories · review before sharing</div>
            <h1 className="display m-0 font-normal text-ink" style={{ fontSize: 22 }}>
              {F.verb} {included.length} insights to {SOURCES[dest].label}.
            </h1>
            <p style={{ fontSize: 13, maxWidth: 720, lineHeight: 1.55 }} className="mt-1 mb-0 text-ink-2">
              Scheduled for <span className="text-ink" >{U.nextBatch.scheduledFor}</span> ({U.cadence}). This destination is{" "}
              <span className="text-ink font-medium" >{F.em}</span>{F.emEnd}
            </p>
          </div>
          <div className="gap-6 pl-6 border-l flex">
            <ShbMini n={included.length} l="will share" accent/>
            <ShbMini n={excluded.size} l="held back"/>
            <ShbMini n={U.contribution.streak} l="week streak" mono/>
          </div>
        </div>
        {/* Destination selector — pick which source receives this batch */}
        <div style={{ gap: 14 }} className="mt-4 flex items-center flex-wrap">
          <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: "0.14em" }}>share to</span>
          <ShbSeg value={dest} onChange={setDest} options={SHARE_DESTS.map(id => ({ id, kanji: SOURCES[id].kanji, label: SOURCES[id].label, color: srcColor(id) }))}/>
          <span className="inline-flex items-center" style={{ fontSize: 11.5, color: c, gap: 6 }}>
            <span className="rounded-full" style={{ width: 6, height: 6, background: c }}/>{KIND[kind].policy}
          </span>
        </div>
      </div>

      {/* Cards */}
      <div style={{ maxWidth: 980 }} className="py-6 px-8 mx-auto flex-1 overflow-auto min-h-0 w-full">
        <div className="gap-2 mb-6 flex flex-col">
          {U.nextBatch.insights.map(ins => {
            const cm = CAT_META[ins.category] || CAT_META.pattern;
            const out = excluded.has(ins.id);
            return (
              <article key={ins.id} style={{ gridTemplateColumns: "24px 1fr auto",
 background: out ? "transparent" : "var(--paper-2)", borderRadius: 6, opacity: out ? 0.5 : 1 }} className="gap-4 py-4 px-4 grid items-start border border-paper-edge">
                <input type="checkbox" checked={!out} onChange={() => toggle(ins.id)}
 style={{ accentColor: "var(--accent)", width: 14, height: 14 }} className="mt-1 cursor-pointer"/>
                <div className="min-w-0" >
                  <div className="gap-2 mb-1 flex items-center flex-wrap">
                    <span className="kanji" style={{ fontSize: 13, color: cm.color }}>{cm.glyph}</span>
                    <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: "0.14em" }}>{cm.label}</span>
                    <ShbSep/><span className="text-ink-3" style={{ fontSize: 11 }}>{ins.evidence} evidence</span>
                    <ShbSep/><span className="mono text-ink-3" style={{ fontSize: 11 }}>conf {Math.round(ins.confidence*100)}%</span>
                  </div>
                  <div style={{ fontSize: 13, lineHeight: 1.4 }} className="mb-1 text-ink font-medium">{ins.title}</div>
                  <div style={{ fontSize: 13, lineHeight: 1.55 }} className="mb-2 text-ink-2">{ins.summary}</div>
                  <div className="flex items-center flex-wrap" style={{ gap: 8 }}>
                    <SrcBadge src={dest} small/>
                    {kind !== "client" && kind !== "global"
                      ? null
                      : <span className="inline-flex items-center text-ink-3 italic" style={{ gap: 5, fontSize: 11 }}>
                          <span className="kanji" style={{ fontSize: 11, color: c }}>{kind === "client" ? "秘" : "匿"}</span>{ins.anonymizationNote}
                        </span>}
                  </div>
                </div>
                <div className="gap-1 flex flex-col items-end">
                  <button style={{ fontSize: 11, borderRadius: 4 }} className="py-1 px-2 bg-transparent border border-paper-edge text-ink-2 cursor-pointer">view source →</button>
                  <span className="mono text-ink-4" style={{ fontSize: 11 }}>{ins.sourceId}</span>
                </div>
              </article>
            );
          })}
        </div>

        {/* Footer — destination-specific confirmation */}
        <div style={{ gap: 10 }} className="pt-4 flex items-center flex-wrap border-t">
          <button style={{ fontSize: 13, background: c, borderRadius: 6, gap: 8 }} className="py-2 px-4 text-paper border-0 cursor-pointer inline-flex items-center">
            <span className="kanji text-paper" style={{ fontSize: 13 }}>{F.glyph}</span>
            {F.verb} {included.length} to {SOURCES[dest].label}
          </button>
          <ShbFlat glyph="待" label="Hold this batch"/>
          <span className="flex-1" />
          <button className="text-ink-3 bg-transparent border-0 cursor-pointer" style={{ fontSize: 11 }}>sharing settings →</button>
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  UPGRADES INBOX — unified, named-source + scope aware
// ═══════════════════════════════════════════════════════════════════
const KIND_META = {
  agent:   { glyph: "作", label: "Agent",   color: "var(--accent)" },
  skill:   { glyph: "技", label: "Skill",   color: "var(--success)" },
  command: { glyph: "令", label: "Command", color: "var(--warning)" },
  lint:    { glyph: "禁", label: "Lint",    color: "var(--ink-2)" },
};

function ObsUpgradesInbox() {
  const U = window.UPGRADES;
  const [source, setSource] = shbS("all");
  const [openId, setOpen] = shbS(U.incoming[0].id);

  // sources actually present in the inbox → filter chips
  const present = shbM(() => {
    const seen = []; U.incoming.forEach(u => { const s = srcOf(u); if (!seen.includes(s)) seen.push(s); }); return seen;
  }, []);
  const filtered = shbM(() => source === "all" ? U.incoming : U.incoming.filter(u => srcOf(u) === source), [source]);
  const item = filtered.find(x => x.id === openId) || filtered[0];

  // hero summary — count per source
  const summary = present.map(s => `${U.incoming.filter(u => srcOf(u) === s).length} from ${SOURCES[s].label}`).join(" · ");

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Upgrades inbox"
 >
      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center">
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>贈</div>
        <div className="flex-1 min-w-0" >
          <div style={{ fontSize: 11, letterSpacing: "0.18em" }} className="mb-1 text-ink-3 uppercase">Observatory · Upgrades inbox</div>
          <h1 className="display m-0 font-normal text-ink" style={{ fontSize: 22 }}>
            {U.incoming.length} upgrades across your sources.
          </h1>
          <p style={{ fontSize: 13, maxWidth: 720, lineHeight: 1.55 }} className="mt-1 mb-0 text-ink-2">
            {summary}. Each item names its source and the scope it applies to — global gifts carry adoption stats; a Dōjō's carry who approved it.
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex">
          <ShbMini n={U.incoming.length} l="waiting"/>
          <ShbMini n={`+${Math.round(U.incoming.reduce((s,u)=>s+u.avgFtrLift,0)*100/U.incoming.length)}%`} l="avg ftr lift" mono accent/>
        </div>
      </div>

      {/* Source filter — All + each named source present */}
      <div style={{ gap: 14 }} className="py-3 px-8 border-b flex items-center flex-wrap">
        <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: "0.14em" }}>source</span>
        <ShbSeg value={source} onChange={setSource}
          options={[{ id: "all", label: "All" }, ...present.map(s => ({ id: s, kanji: SOURCES[s].kanji, label: SOURCES[s].label, color: srcColor(s) }))]}/>
        <span className="flex-1" />
        <span className="text-ink-4" style={{ fontSize: 11 }}>{filtered.length} of {U.incoming.length}</span>
      </div>

      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: "340px 1fr" }}>
        {/* List */}
        <aside className="py-2 border-r overflow-auto">
          {filtered.map(u => {
            const km = KIND_META[u.kind]; const open = openId === u.id; const s = srcOf(u); const meta = UPG_SOURCE[u.id] || {};
            return (
              <button key={u.id} onClick={() => setOpen(u.id)} style={{
 background: open ? "var(--paper-2)" : "transparent",
 borderLeft: open ? "2px solid var(--accent)" : "2px solid transparent", gap: 6 }} className="py-3 px-4 w-full text-left cursor-pointer flex flex-col">
                <div className="flex items-center" style={{ gap: 8 }}>
                  <span className="kanji" style={{ fontSize: 13, color: km.color }}>{km.glyph}</span>
                  <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: "0.14em" }}>{km.label}</span>
                  <span className="flex-1" />
                  <SrcBadge src={s} small/>
                </div>
                <div className="mono" style={{ fontSize: 11, color: open ? "var(--ink)" : "var(--ink-2)", lineHeight: 1.4 }}>{u.name}</div>
                <div className="flex items-center flex-wrap text-ink-3" style={{ gap: 8, fontSize: 11 }}>
                  {SOURCES[s].kind === "global"
                    ? <span>{u.contributors} sources · {u.adoptions} installs</span>
                    : <span className="inline-flex items-center" style={{ gap: 5 }}><span className="kanji" style={{ color: srcColor(s) }}>認</span>{meta.approver}</span>}
                  <ShbSep/><span className="mono text-success" >+{Math.round(u.avgFtrLift*100)}%</span>
                  <ScopePill label={scopeLabel(u)}/>
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main className="py-8 px-12 overflow-auto">
          {item && <UpgInboxDetail item={item}/>}
        </main>
      </div>
    </div>
  );
}

function UpgInboxDetail({ item }) {
  const km = KIND_META[item.kind]; const s = srcOf(item); const meta = UPG_SOURCE[item.id] || {}; const kind = SOURCES[s].kind;
  return (
    <div style={{ maxWidth: 720 }} className="mx-auto">
      <div style={{ gap: 12, fontSize: 11, letterSpacing: "0.12em" }} className="mb-3 flex items-center flex-wrap text-ink-3 uppercase">
        <span className="kanji" style={{ fontSize: 13, color: km.color, letterSpacing: 0 }}>{km.glyph}</span>
        <span>{km.label}</span><ShbSep/><span>{item.maturity}</span>
        <span className="flex-1" /><SrcBadge src={s}/>
      </div>
      <h2 className="display mt-0 mb-2 font-light text-ink" style={{ fontSize: 28, lineHeight: 1.2, letterSpacing: "-0.015em" }}>{item.title}</h2>
      <div style={{ gap: 10 }} className="mb-6 flex items-center flex-wrap">
        <span className="mono text-ink-3" style={{ fontSize: 13 }}>{item.name}</span>
        <ScopePill label={"applies to " + scopeLabel(item)}/>
      </div>
      <p style={{ fontSize: 15, lineHeight: 1.65 }} className="mt-0 mb-6 text-ink-2">{item.summary}</p>

      {/* provenance panel — governance (named Dōjō) vs adoption (global) */}
      <div style={{ borderLeft: `2px solid ${srcColor(s)}`, borderRadius: 6 }} className="py-3 px-4 mb-6 bg-paper-2 border border-paper-edge">
        <div style={{ fontSize: 11, letterSpacing: "0.14em" }} className="mb-1 text-ink-3 uppercase">
          {kind === "global" ? "Adopted across the network" : "Approved in " + SOURCES[s].label}
        </div>
        <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.6 }}>
          {kind === "global"
            ? <><span className="mono">{item.adoptions}</span> installs from <span className="mono">{item.contributors}</span> contributing senseis · {item.sourceModel}. Anonymized aggregate — no identities attached.</>
            : <>Reviewed and approved by <span className="mono">{meta.approver}</span>{meta.team ? " (" + meta.team + " team)" : ""} · {meta.approvedOn}. {kind === "client" ? "Source-dereferenced under the client's confidentiality policy." : "Governed by " + SOURCES[s].label + "'s scope policies — installs are logged to its audit trail."}</>}
        </div>
      </div>

      <div style={{ borderLeft: "2px solid var(--accent)", borderRadius: 6 }} className="py-3 px-4 mb-6 bg-paper-2 border border-paper-edge">
        <div style={{ fontSize: 11, letterSpacing: "0.14em" }} className="mb-1 text-ink-3 uppercase">Why for you</div>
        <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.6 }}>{item.why}</div>
      </div>

      <div className="mb-6">
        <div style={{ fontSize: 11, letterSpacing: "0.14em" }} className="mb-2 text-ink-3 uppercase">What it does</div>
        <ol className="gap-1 m-0 pl-4 flex flex-col">
          {item.preview.map((p, i) => <li className="text-ink-2" key={i} style={{ fontSize: 13, lineHeight: 1.5 }}>{p}</li>)}
        </ol>
      </div>

      {item.conflicts.length > 0 && (
        <div className="mb-6">
          <div style={{ fontSize: 11, letterSpacing: "0.14em" }} className="mb-2 text-warning uppercase">Touches existing memory</div>
          {item.conflicts.map((cf, i) => (
            <div key={i} style={{ fontSize: 13, borderRadius: 4 }} className="py-2 px-3 mb-1 text-ink-2 bg-warning-soft">
              <span className="mono text-ink" >{cf.id}</span>{" — "}{cf.note}
            </div>
          ))}
        </div>
      )}

      <div style={{ gap: 8 }} className="pt-4 flex items-center border-t">
        <button style={{ fontSize: 13, borderRadius: 6, gap: 8 }} className="py-2 px-4 bg-ink text-paper border-0 cursor-pointer inline-flex items-center">
          <span className="kanji text-accent" style={{ fontSize: 13 }}>受</span>
          Install for {item.relevantProjects.join(" · ")}
        </button>
        <ShbFlat glyph="試" label="Preview in sandbox"/>
        <ShbFlat glyph="後" label="Defer"/>
        <span className="flex-1" />
        <ShbFlat glyph="納" label="Dismiss" subtle/>
      </div>
    </div>
  );
}

Object.assign(window, { ObsShareReviewHub, ObsUpgradesInbox });
