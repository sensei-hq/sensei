// Dōjō · Extensions catalog — the team's shared, governed catalog of packaged
// behaviors (skills · commands · agents · personas · hooks · plugins).
//
// Uplifts the Observatory's per-developer Extensions browser to the team
// surface: an org curates and approves extensions, scopes them org → team →
// project, and sees adoption across the team. A developer who joins inherits
// the approved set. Reads window.EXT_DATA; reuses DojoHead / DojoChip.

const { useState: exS, useMemo: exM } = React;

// Dōjō framing for provenance + governance status, derived from EXT_DATA.
const EX_SCOPE = { global: { k: "社", label: "Org" }, either: { k: "組", label: "Team" }, project: { k: "件", label: "Project" } };

function DojoExtensions({ mobile = false }) {
  const E = window.EXT_DATA || { kinds: [], extensions: [] };
  const [kind, setKind] = exS("all");
  const [tab, setTab] = exS("approved"); // approved | proposed
  const kindMeta = exM(() => { const m = {}; E.kinds.forEach(k => m[k.id] = k); return m; }, [E.kinds]);

  // In the Dōjō, an extension is "approved" (curated into the team catalog) or
  // "proposed" (a member published it locally, awaiting a maintainer).
  const withStatus = exM(() => E.extensions.map((e, i) => ({
    ...e,
    status: e.source === "you" || e.source === "local" ? "proposed" : "approved",
    teams: (e.pinnedTo || []).length + (e.scope === "global" ? 3 : 0),
  })), [E.extensions]);

  const filtered = withStatus.filter(e => (tab === "proposed" ? e.status === "proposed" : e.status === "approved") && (kind === "all" || e.kind === kind));
  const proposedCount = withStatus.filter(e => e.status === "proposed").length;

  const Stat = ({ n, l }) => (
    <div className="text-right" >
      <div className="display text-2xl font-light text-ink" style={{ lineHeight: 1 }}>{n}</div>
      <div className="text-xs uppercase text-ink-faint mt-1" style={{ letterSpacing: ".12em" }}>{l}</div>
    </div>
  );

  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="庫" eyebrow="Team · catalog" title="Extensions catalog"
        sub="The behaviors your team shares — skills, commands, agents, personas, hooks and plugins. Curated and approved once, scoped org → team → project, and inherited by everyone who joins."
        right={<div className="flex gap-4" >
          <Stat n={withStatus.filter(e => e.status === "approved").length} l="approved" />
          <Stat n={proposedCount} l="proposed" />
        </div>} />

      {/* tabs + kind filter */}
      <div className="shrink-0 border-b flex items-center flex-wrap gap-y-2" style={{ padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-6)", gap: mobile ? "var(--space-2)" : "var(--space-4)" }}>
        <div className="inline-flex bg-paper-mute rounded-lg p-1 gap-1" >
          {[{ id: "approved", label: "Team catalog" }, { id: "proposed", label: `Proposed · ${proposedCount}` }].map(t => {
            const on = tab === t.id;
            return (
              <button className="border-0 cursor-pointer rounded py-1 px-3 text-sm" key={t.id} onClick={() => setTab(t.id)} style={{ fontWeight: on ? 600 : 400, fontFamily: "inherit",
 background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
 boxShadow: on ? "var(--shadow-sm)" : "none" }}>{t.label}</button>
            );
          })}
        </div>
        <div className="flex gap-2 flex-wrap" >
          {[{ id: "all", kanji: "全", label: "All" }, ...E.kinds].map(k => {
            const on = kind === k.id;
            return (
              <button className="inline-flex items-center gap-1 cursor-pointer rounded-full py-1 px-3 text-xs" key={k.id} onClick={() => setKind(k.id)} style={{
 border: on ? "1px solid var(--ink)" : "var(--hairline)", fontFamily: "inherit",
 background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)" }}>
                <span className="kanji text-sm" style={{ color: on ? "var(--paper)" : "var(--accent)" }}>{k.kanji}</span>{k.label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-3 py-16 px-0 text-ink-mute" >
            <span className="kanji text-3xl text-ink-faint" >空</span>
            <div className="text-sm text-ink-soft" >{tab === "proposed" ? "Nothing waiting to curate." : "No extensions in the catalog yet."}</div>
          </div>
        ) : (
          <div className="grid gap-3" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fill, minmax(360px, 1fr))" }}>
            {filtered.map(e => {
              const km = kindMeta[e.kind] || { kanji: "・", label: e.kind };
              const sc = EX_SCOPE[e.scope] || EX_SCOPE.either;
              const proposed = e.status === "proposed";
              return (
                <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4 flex flex-col gap-2" key={e.id} >
                  <div className="flex items-start gap-3" >
                    <span className="rounded-lg shrink-0 bg-paper-mute flex items-center justify-center" style={{ width: 38, height: 38 }}>
                      <span className="kanji text-lg text-accent" >{km.kanji}</span>
                    </span>
                    <div className="flex-1 min-w-0" >
                      <div className="flex items-center gap-2 flex-wrap" >
                        <span className="text-base text-ink font-semibold" >{e.name}</span>
                        <span className="text-xs uppercase text-ink-faint font-semibold" style={{ letterSpacing: ".1em" }}>{km.label}</span>
                      </div>
                      <div className="mono text-xs text-ink-faint mt-1" >{e.author} · v{e.version}</div>
                    </div>
                    {proposed
                      ? <DojoChip tone="var(--accent)" soft="var(--accent-soft)">proposed</DojoChip>
                      : <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ approved</DojoChip>}
                  </div>
                  <div className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>{e.desc}</div>
                  <div className="flex items-center gap-2 flex-wrap" >
                    <DojoChip tone="var(--ink-soft)">{sc.k} {sc.label}</DojoChip>
                    <span className="mono text-xs text-ink-mute" >{e.evidence} evidence · ★ {e.stars}</span>
                  </div>
                  <div className="flex items-center gap-2 mt-1 pt-2" style={{ borderTop: "1px solid var(--paper-edge)" }}>
                    {proposed ? (
                      <span className="text-xs text-ink-mute" >Published by <b className="font-semibold text-ink-soft" >{e.author}</b> · awaiting curation</span>
                    ) : (
                      <span className="text-xs text-ink-mute" >Adopted in <b className="font-semibold text-ink-soft" >{e.teams}</b> {e.teams === 1 ? "project" : "projects"} · in the team catalog</span>
                    )}
                    <span className="flex-1" />
                    {proposed
                      ? <DojoBtn size="sm" kanji="決">Review</DojoBtn>
                      : <DojoBtn size="sm" variant="ghost">{sc.label} ▾</DojoBtn>}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

window.DojoExtensions = DojoExtensions;
