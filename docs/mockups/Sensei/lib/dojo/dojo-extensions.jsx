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
    <div style={{ textAlign: "right" }}>
      <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{n}</div>
      <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{l}</div>
    </div>
  );

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="庫" eyebrow="Team · catalog" title="Extensions catalog"
        sub="The behaviors your team shares — skills, commands, agents, personas, hooks and plugins. Curated and approved once, scoped org → team → project, and inherited by everyone who joins."
        right={<div style={{ display: "flex", gap: "var(--space-4)" }}>
          <Stat n={withStatus.filter(e => e.status === "approved").length} l="approved" />
          <Stat n={proposedCount} l="proposed" />
        </div>} />

      {/* tabs + kind filter */}
      <div style={{ flexShrink: 0, borderBottom: "var(--hairline)", padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-5)", display: "flex", alignItems: "center", gap: mobile ? "var(--space-2)" : "var(--space-4)", flexWrap: "wrap", rowGap: "var(--space-2)" }}>
        <div style={{ display: "inline-flex", background: "var(--paper-mute)", borderRadius: "var(--radius-lg)", padding: "var(--space-1)", gap: "var(--space-1)" }}>
          {[{ id: "approved", label: "Team catalog" }, { id: "proposed", label: `Proposed · ${proposedCount}` }].map(t => {
            const on = tab === t.id;
            return (
              <button key={t.id} onClick={() => setTab(t.id)} style={{ border: "none", cursor: "pointer", borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-3)",
                fontSize: "var(--text-sm)", fontWeight: on ? 600 : 400, fontFamily: "inherit",
                background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
                boxShadow: on ? "var(--shadow-sm)" : "none" }}>{t.label}</button>
            );
          })}
        </div>
        <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
          {[{ id: "all", kanji: "全", label: "All" }, ...E.kinds].map(k => {
            const on = kind === k.id;
            return (
              <button key={k.id} onClick={() => setKind(k.id)} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", cursor: "pointer",
                border: on ? "1px solid var(--ink)" : "var(--hairline)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-3)", fontSize: "var(--text-xs)", fontFamily: "inherit",
                background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-sm)", color: on ? "var(--paper)" : "var(--accent)" }}>{k.kanji}</span>{k.label}
              </button>
            );
          })}
        </div>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
        {filtered.length === 0 ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "var(--space-3)", padding: "var(--space-8) 0", color: "var(--ink-mute)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-3xl)", color: "var(--ink-faint)" }}>空</span>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>{tab === "proposed" ? "Nothing waiting to curate." : "No extensions in the catalog yet."}</div>
          </div>
        ) : (
          <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fill, minmax(360px, 1fr))", gap: "var(--space-3)" }}>
            {filtered.map(e => {
              const km = kindMeta[e.kind] || { kanji: "・", label: e.kind };
              const sc = EX_SCOPE[e.scope] || EX_SCOPE.either;
              const proposed = e.status === "proposed";
              return (
                <div key={e.id} style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                  <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-3)" }}>
                    <span style={{ width: 38, height: 38, borderRadius: "var(--radius-lg)", flexShrink: 0, background: "var(--paper-mute)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                      <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)" }}>{km.kanji}</span>
                    </span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
                        <span style={{ fontSize: "var(--text-base)", color: "var(--ink)", fontWeight: 600 }}>{e.name}</span>
                        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600 }}>{km.label}</span>
                      </div>
                      <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{e.author} · v{e.version}</div>
                    </div>
                    {proposed
                      ? <DojoChip tone="var(--accent)" soft="var(--accent-soft)">proposed</DojoChip>
                      : <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ approved</DojoChip>}
                  </div>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>{e.desc}</div>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
                    <DojoChip tone="var(--ink-soft)">{sc.k} {sc.label}</DojoChip>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{e.evidence} evidence · ★ {e.stars}</span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-1)", paddingTop: "var(--space-2)", borderTop: "1px solid var(--paper-edge)" }}>
                    {proposed ? (
                      <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>Published by <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>{e.author}</b> · awaiting curation</span>
                    ) : (
                      <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>Adopted in <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>{e.teams}</b> {e.teams === 1 ? "project" : "projects"} · in the team catalog</span>
                    )}
                    <span style={{ flex: 1 }} />
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
