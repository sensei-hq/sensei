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

function DojoExtensions() {
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
    approver: ["Keiko T.", "Marco D.", "Sven K."][i % 3],
  })), [E.extensions]);

  const filtered = withStatus.filter(e => (tab === "proposed" ? e.status === "proposed" : e.status === "approved") && (kind === "all" || e.kind === kind));
  const proposedCount = withStatus.filter(e => e.status === "proposed").length;

  const Stat = ({ n, l }) => (
    <div style={{ textAlign: "right" }}>
      <div className="display" style={{ fontSize: 26, fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{n}</div>
      <div style={{ fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-4)", marginTop: 4 }}>{l}</div>
    </div>
  );

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="庫" eyebrow="Team · catalog" title="Extensions catalog"
        sub="The behaviors your team shares — skills, commands, agents, personas, hooks and plugins. Curated and approved once, scoped org → team → project, and inherited by everyone who joins."
        right={<div style={{ display: "flex", gap: 20 }}>
          <Stat n={withStatus.filter(e => e.status === "approved").length} l="approved" />
          <Stat n={proposedCount} l="proposed" />
        </div>} />

      {/* tabs + kind filter */}
      <div style={{ flexShrink: 0, borderBottom: "var(--hairline)", padding: "12px 28px", display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
        <div style={{ display: "inline-flex", background: "var(--paper-3)", borderRadius: 8, padding: 3, gap: 2 }}>
          {[{ id: "approved", label: "Team catalog" }, { id: "proposed", label: `Proposed · ${proposedCount}` }].map(t => {
            const on = tab === t.id;
            return (
              <button key={t.id} onClick={() => setTab(t.id)} style={{ border: "none", cursor: "pointer", borderRadius: 6, padding: "6px 13px",
                fontSize: 12.5, fontWeight: on ? 600 : 400, fontFamily: "inherit",
                background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-3)",
                boxShadow: on ? "0 1px 2px rgba(0,0,0,0.08)" : "none" }}>{t.label}</button>
            );
          })}
        </div>
        <div style={{ display: "flex", gap: 7, flexWrap: "wrap" }}>
          {[{ id: "all", kanji: "全", label: "All" }, ...E.kinds].map(k => {
            const on = kind === k.id;
            return (
              <button key={k.id} onClick={() => setKind(k.id)} style={{ display: "inline-flex", alignItems: "center", gap: 6, cursor: "pointer",
                border: on ? "1px solid var(--ink)" : "var(--hairline)", borderRadius: 999, padding: "5px 12px", fontSize: 12, fontFamily: "inherit",
                background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-2)" }}>
                <span className="kanji" style={{ fontSize: 13, color: on ? "var(--paper)" : "var(--accent)" }}>{k.kanji}</span>{k.label}
              </button>
            );
          })}
        </div>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        {filtered.length === 0 ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 12, padding: "60px 0", color: "var(--ink-3)" }}>
            <span className="kanji" style={{ fontSize: 44, color: "var(--ink-4)" }}>空</span>
            <div style={{ fontSize: 14, color: "var(--ink-2)" }}>{tab === "proposed" ? "Nothing waiting to curate." : "No extensions in the catalog yet."}</div>
          </div>
        ) : (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(360px, 1fr))", gap: 14 }}>
            {filtered.map(e => {
              const km = kindMeta[e.kind] || { kanji: "・", label: e.kind };
              const sc = EX_SCOPE[e.scope] || EX_SCOPE.either;
              const proposed = e.status === "proposed";
              return (
                <div key={e.id} style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px", display: "flex", flexDirection: "column", gap: 10 }}>
                  <div style={{ display: "flex", alignItems: "flex-start", gap: 11 }}>
                    <span style={{ width: 38, height: 38, borderRadius: 10, flexShrink: 0, background: "var(--paper-3)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                      <span className="kanji" style={{ fontSize: 19, color: "var(--accent)" }}>{km.kanji}</span>
                    </span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                        <span style={{ fontSize: 14.5, color: "var(--ink)", fontWeight: 600 }}>{e.name}</span>
                        <span style={{ fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600 }}>{km.label}</span>
                      </div>
                      <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 2 }}>{e.author} · v{e.version}</div>
                    </div>
                    {proposed
                      ? <DojoChip tone="var(--accent)" soft="var(--accent-soft)">proposed</DojoChip>
                      : <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ approved</DojoChip>}
                  </div>
                  <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>{e.desc}</div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                    <DojoChip tone="var(--ink-2)">{sc.k} {sc.label}</DojoChip>
                    <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)" }}>{e.evidence} evidence · ★ {e.stars}</span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 2, paddingTop: 10, borderTop: "1px solid var(--edge)" }}>
                    {proposed ? (
                      <span style={{ fontSize: 11.5, color: "var(--ink-3)" }}>Published by <b style={{ fontWeight: 600, color: "var(--ink-2)" }}>{e.author}</b> · awaiting curation</span>
                    ) : (
                      <span style={{ fontSize: 11.5, color: "var(--ink-3)" }}>Adopted in <b style={{ fontWeight: 600, color: "var(--ink-2)" }}>{e.teams}</b> {e.teams === 1 ? "project" : "projects"} · approved by {e.approver}</span>
                    )}
                    <span style={{ flex: 1 }} />
                    {proposed
                      ? <button style={{ fontSize: 12, background: "var(--ink)", color: "var(--paper)", border: "none", borderRadius: 7, padding: "6px 13px", cursor: "pointer", fontFamily: "inherit", display: "inline-flex", alignItems: "center", gap: 6 }}><span className="kanji" style={{ fontSize: 12, color: "var(--accent)" }}>決</span> Review</button>
                      : <button style={{ fontSize: 12, background: "var(--paper)", color: "var(--ink-2)", border: "var(--hairline)", borderRadius: 7, padding: "6px 13px", cursor: "pointer", fontFamily: "inherit" }}>Scope ▾</button>}
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
