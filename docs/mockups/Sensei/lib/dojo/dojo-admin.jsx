// Dōjō · Admin console — run the Dōjō.
// Panels: Overview · Monitor · Members · Scopes. Job: stand it up, connect
// identity (OIDC/SAML), provision members, set scopes & policies.
// Reuses the shared frame from dojo-shared.jsx (DojoRoleShell / DojoHead /
// DojoChip / OriginChip / Confidence) and primitives (Avatar / Sparkline).

const { useState: daS } = React;

const ADMIN_NAV = [
  { group: "Health", items: [
    { id: "overview", kanji: "全", label: "Overview" },
    { id: "monitor",  kanji: "観", label: "Monitor" },
  ]},
  { group: "Org · manage", manage: true, items: [
    { id: "governance", kanji: "掟", label: "Governance" },
    { id: "members", kanji: "任", label: "Members & roles" },
    { id: "identity", kanji: "鍵", label: "Identity & SSO" },
    { id: "scopes",  kanji: "規", label: "Scopes & policies" },
    { id: "billing", kanji: "円", label: "Plan & billing" },
  ]},
];
const ADMIN_SECTIONS = ["overview", "monitor", "members", "scopes", "governance", "billing", "identity"];

/* ─── Overview ───────────────────────────────────────────── */
function DojoOverview({ go, mobile = false }) {
  const D = window.DOJO, m = D.metrics;
  const [retract, setRetract] = daS(null);
  const [retracted, setRetracted] = daS([]);
  const published = [
    { kanji: "守", title: "Never log refresh tokens", scope: "Company", adoption: 0.92, delta: 6, status: "active" },
    { kanji: "紋", title: "Idempotency key on money-moving mutations", scope: "Team · Payments", adoption: 0.78, delta: 9, status: "active" },
    { kanji: "問", title: "Prefer optimistic UI for list mutations", scope: "Stack · React", adoption: 0.41, delta: -3, status: "flagged" },
  ];
  const Metric = ({ kanji, label, value, sub, children, onClick }) => (
    <div onClick={onClick} style={{ flex: 1, background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)",
      padding: "var(--space-4) var(--space-4)", cursor: onClick ? "pointer" : "default", minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)" }}>{label}</span>
      </div>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: "var(--space-2)" }}>
        <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{value}</div>
        {children}
      </div>
      {sub && <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-2)" }}>{sub}</div>}
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)", position: "relative" }}>
      <DojoHead mobile={mobile} kanji="全" eyebrow="Acme Corp · Dōjō" title="The shared mind, governed."
        sub="What your org has learned — triaged, approved, and routed to the scopes that need it."
        right={<div style={{ textAlign: "right", fontSize: "var(--text-xs)", color: "var(--ink-mute)", fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div>{D.org.scopes} scopes · {D.org.repos} repos</div>
          <div style={{ color: "var(--success)" }}>{m.incidents} confidentiality incidents</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
        <div style={{ display: "flex", flexDirection: mobile ? "column" : "row", gap: "var(--space-3)" }}>
          <Metric kanji="門" label="Pending triage" value={m.pendingTriage} sub="across 4 scopes · oldest 3d">
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>maintainers →</span>
          </Metric>
          <Metric kanji="共" label="Contributions · 7d" value={m.contribWeek}>
            <Sparkline data={m.contribSpark} width={92} height={30} color="var(--accent)" fill="var(--accent-soft)" />
          </Metric>
          <Metric kanji="決" label="Approved · 7d" value={m.approvedWeek} sub="published to matching scopes" />
          <Metric kanji="果" label="Adoption lift" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} sub="FTR across adopting scopes">
            <Sparkline data={m.ftrSpark} width={92} height={30} color="var(--success)" />
          </Metric>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1.4fr 1fr", gap: "var(--space-4)", marginTop: "var(--space-4)" }}>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)" }}>Top of the triage queue</span>
              <span style={{ flex: 1 }} />
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>maintainers own review</span>
            </div>
            {D.queue.slice(0, 4).map((c, i) => (
              <div key={c.id} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center",
                width: "100%", textAlign: "left", padding: "var(--space-3) var(--space-4)", borderBottom: i < 3 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 20, textAlign: "center" }}>{c.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.title}</div>
                  <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center" }}>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{c.scope}</span>
                    <OriginChip origin={c.origin} />
                  </div>
                </div>
                <Confidence v={c.confidence} w={56} />
              </div>
            ))}
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
            <div style={{ background: "var(--paper-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--success)" }}>盾</span>
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)" }}>Confidentiality</span>
              </div>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.55 }}>
                <b style={{ fontWeight: 600 }}>{m.anonymized}</b> client lessons auto-anonymized this week ·
                <span style={{ color: "var(--success)" }}> 0 incidents</span>. Sources dropped automatically; only flagged exceptions reach a lead.
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-2)", paddingTop: "var(--space-2)", borderTop: "1px solid var(--paper-edge)" }}>
                <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success)" }} />
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", textTransform: "uppercase", letterSpacing: ".06em" }}>Leak-guard armed</span>
                <span style={{ flex: 1 }} />
                <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>alerts shared to the client lead <span style={{ color: "var(--accent)" }}>→</span></span>
              </div>
            </div>
            <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", flex: 1 }}>
              <div style={{ padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)", fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)" }}>Recent activity</div>
              <div style={{ padding: "var(--space-1) 0" }}>
                {D.activity.map((a, i) => (
                  <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-2)", alignItems: "start", padding: "var(--space-2) var(--space-4)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", width: 16, textAlign: "center",
                                  color: a.tone === "success" ? "var(--success)" : a.tone === "accent" ? "var(--accent)" : "var(--ink-mute)" }}>{a.kanji}</span>
                    <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.45 }}>{a.text}</span>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{a.when}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
        <div style={{ marginTop: "var(--space-4)" }}>
          <div style={{ display: "flex", alignItems: "center", marginBottom: "var(--space-2)" }}>
            <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Published · adoption &amp; health</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginLeft: "var(--space-2)" }}>the Impact loop, scoped to the org</span>
          </div>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", overflowX: mobile ? "auto" : "hidden" }}>
            {published.map((p, i) => {
              const neg = p.delta < 0;
              return (
              <div key={p.title} style={{ display: "grid", gridTemplateColumns: "auto 1fr 150px 92px 168px", gap: "var(--space-3)", alignItems: "center",
                            padding: "var(--space-3) var(--space-4)", borderBottom: i < published.length - 1 ? "1px solid var(--paper-edge)" : "none",
                            background: neg ? "var(--warning-soft)" : "transparent" }}>
                <span className="kanji" style={{ fontSize: "var(--text-lg)", color: neg ? "var(--warning)" : "var(--accent)", width: 20, textAlign: "center" }}>{p.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.title}</div>
                  <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{p.scope}</div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                  <div style={{ flex: 1, height: 4, borderRadius: "var(--radius-sm)", background: "var(--paper-mute)", overflow: "hidden" }}>
                    <div style={{ width: (p.adoption * 100) + "%", height: "100%", background: "var(--accent)", borderRadius: "var(--radius-sm)" }} />
                  </div>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{Math.round(p.adoption * 100)}%</span>
                </div>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: neg ? "var(--warning)" : "var(--success)", textAlign: "right" }}>
                  {neg ? "" : "+"}{p.delta}pp FTR
                </span>
                <div style={{ display: "flex", justifyContent: "flex-end" }}>
                  {retracted.includes(p.title)
                    ? <DojoChip tone="var(--danger)" soft="var(--danger-soft)">退 retracted</DojoChip>
                    : neg
                    ? <button onClick={() => setRetract(p)} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)",
                              border: "1px solid var(--danger-edge)", background: "var(--paper)", color: "var(--danger)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit" }}>
                        <span className="kanji" style={{ fontSize: "var(--text-xs)" }}>退</span> Retract downstream
                      </button>
                    : <DojoChip tone="var(--success)" soft="var(--success-soft)">active</DojoChip>}
                </div>
              </div>
              );
            })}
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-2) var(--space-4)", borderTop: "1px solid var(--paper-edge)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.45 }}>
              <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--warning)" }}>退</span>
              <span>Lifecycle <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>active → deprecated → retracted</b>. Negative impact is flagged automatically; one-click retract pulls a teaching back and notifies adopters.</span>
            </div>
          </div>
        </div>
      </div>
      {retract && (
        <div onClick={() => setRetract(null)} style={{ position: "absolute", inset: 0, zIndex: 60, background: "var(--scrim)",
              display: "flex", alignItems: "center", justifyContent: "center", padding: "var(--space-5)" }}>
          <div onClick={e => e.stopPropagation()} style={{ width: 480, maxWidth: "100%", background: "var(--paper)", border: "1px solid var(--danger-edge)",
                borderRadius: "var(--radius-lg)", boxShadow: "var(--shadow-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--danger)" }}>退</span>
              <div>
                <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--danger)", fontWeight: 700 }}>Retract downstream</div>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", marginTop: "var(--space-1)" }}>{retract.title}</div>
              </div>
            </div>
            <div style={{ padding: "var(--space-4)" }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, marginBottom: "var(--space-3)" }}>
                This teaching is showing <b style={{ fontWeight: 600, color: "var(--warning)" }}>{retract.delta}pp FTR</b> in <b style={{ fontWeight: 600, color: "var(--ink)" }}>{retract.scope}</b>. Retracting pulls it from every adopting scope and notifies adopters — the lesson moves to <b style={{ fontWeight: 600 }}>retracted</b> in the audit trail.
              </div>
              <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", marginBottom: "var(--space-4)" }}>
                <div style={{ padding: "var(--space-2) var(--space-3)", borderBottom: "var(--hairline)", fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>What this affects</div>
                {[["場", "Adopting scopes", "Company · 3 teams · 11 repos"], ["人", "Adopters notified", "24 contributors — with the reason"], ["録", "Audit", "active → deprecated → retracted, hash-chained"]].map(([k, l, v]) => (
                  <div key={l} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-2)", alignItems: "center", padding: "var(--space-2) var(--space-3)", borderBottom: "1px solid var(--paper-edge)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", width: 16, textAlign: "center" }}>{k}</span>
                    <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{l}</span>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textAlign: "right" }}>{v}</span>
                  </div>
                ))}
              </div>
              <div style={{ display: "flex", justifyContent: "flex-end", gap: "var(--space-2)" }}>
                <button onClick={() => setRetract(null)} style={{ padding: "var(--space-2) var(--space-4)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", cursor: "pointer", fontFamily: "inherit" }}>Cancel</button>
                <button onClick={() => { setRetracted(a => [...a, retract.title]); setRetract(null); }} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", padding: "var(--space-2) var(--space-4)", borderRadius: "var(--radius-lg)", border: "none", background: "var(--danger)", color: "var(--paper)", fontSize: "var(--text-sm)", fontWeight: 500, cursor: "pointer", fontFamily: "inherit" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--paper)" }}>退</span> Retract &amp; notify
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/* ─── Monitor · hive-mind health ─────────────────────────── */
function DojoMonitor({ go, mobile = false }) {
  const D = window.DOJO, m = D.metrics;
  const days = ["M", "T", "W", "T", "F", "S", "S"];
  const weeks = m.contribSpark.map((c, i) => ({ c, a: Math.max(0, Math.round(c * (0.62 + i * 0.025))) }));
  const maxBar = Math.max(...weeks.map(w => w.c));
  const approvalRate = Math.round((m.approvedWeek / m.contribWeek) * 100);
  const sevTone = { high: "var(--danger)", med: "var(--warning)", low: "var(--ink-mute)" };
  const sevSoft = { high: "var(--danger-soft)", med: "var(--warning-soft)", low: "var(--paper-mute)" };
  const alerts = [
    { sev: "high", k: "警", title: "Anomalous outbound volume — Initech scope", state: "quarantined", client: "Initech", when: "2h",
      note: "14 lessons queued outbound in 5 min; auto-quarantined and held pending a lead review." },
    { sev: "med", k: "盾", title: "Named entity survived anonymize — Globex", state: "exception", client: "Globex", when: "5h",
      note: "A tenant-id naming scheme cleared the classifier; routed to the client lead's exception queue." },
    { sev: "low", k: "盾", title: "Rare-context lesson flagged — Globex", state: "exception", client: "Globex", when: "1d",
      note: "Low k-anonymity; held for confirmation rather than weakened." },
  ];
  const Signal = ({ kanji, label, value, sub, tone = "var(--ink)", children }) => (
    <div style={{ flex: 1, background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)" }}>{label}</span>
      </div>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: "var(--space-2)" }}>
        <div className="display" style={{ fontSize: "var(--text-3xl)", fontWeight: 300, lineHeight: 1, color: tone }}>{value}</div>
        {children}
      </div>
      <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-2)", lineHeight: 1.4 }}>{sub}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)", position: "relative" }}>
      <DojoHead mobile={mobile} kanji="観" eyebrow="Org · monitor" title="Hive-mind health"
        sub="Three headline signals — throughput, adoption, and leak-guard — read over the full audit trail. Anomalies surface here and flow straight into the client lead's incident view."
        right={<div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Last 7 days ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)",
                        color: "var(--success)", background: "var(--success-soft)", border: "1px solid var(--success-edge)",
                        borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)" }}>● leak-guard armed</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
        <div style={{ display: "flex", flexDirection: mobile ? "column" : "row", gap: "var(--space-3)" }}>
          <Signal kanji="通" label="Throughput · 7d" value={`${m.approvedWeek}/${m.contribWeek}`}
            sub={`${approvalRate}% approved · ${m.contribWeek} contributed, ${m.approvedWeek} published`}>
            <Sparkline data={m.contribSpark} width={96} height={32} color="var(--accent)" fill="var(--accent-soft)" />
          </Signal>
          <Signal kanji="果" label="Adoption rate" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} tone="var(--success)"
            sub="FTR lift across adopting scopes, trending up week over week">
            <Sparkline data={m.ftrSpark} width={96} height={32} color="var(--success)" />
          </Signal>
          <Signal kanji="盾" label="Leak-guard · 7d" value={alerts.length}
            sub={`${m.incidents} confidentiality incidents · ${m.anonymized} sources dropped`}>
            <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "var(--space-1)" }}>
              <div style={{ display: "flex", gap: "var(--space-1)" }}>
                {alerts.map((a, i) => <span key={i} style={{ width: 9, height: 9, borderRadius: "50%", background: sevTone[a.sev] }} />)}
              </div>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textTransform: "uppercase", letterSpacing: ".06em" }}>2 exceptions · 1 held</span>
            </div>
          </Signal>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(340px, 1fr))", gap: "var(--space-4)", marginTop: "var(--space-4)" }}>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)", flexWrap: "wrap" }}>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Contributions vs. approvals</span>
              <span style={{ flex: 1 }} />
              <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}><span style={{ width: 9, height: 9, borderRadius: "var(--radius-sm)", background: "var(--accent)", opacity: .85 }} /> contributed</span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}><span style={{ width: 9, height: 9, borderRadius: "var(--radius-sm)", background: "var(--success)" }} /> approved</span>
            </div>
            <div style={{ padding: "var(--space-4) var(--space-4) var(--space-4)" }}>
              <div style={{ display: "flex", alignItems: "flex-end", gap: "var(--space-4)", height: 122 }}>
                {weeks.map((w, i) => (
                  <div key={i} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: "var(--space-2)" }}>
                    <div style={{ height: 96, width: "100%", display: "flex", alignItems: "flex-end", justifyContent: "center", gap: "var(--space-1)" }}>
                      <div style={{ width: 11, height: Math.round(w.c / maxBar * 96), background: "var(--accent)", opacity: .85, borderRadius: "var(--radius-sm) var(--radius-sm) 0 0" }} />
                      <div style={{ width: 11, height: Math.round(w.a / maxBar * 96), background: "var(--success)", borderRadius: "var(--radius-sm) var(--radius-sm) 0 0" }} />
                    </div>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{days[i]}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", display: "flex", flexDirection: "column" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>警</span>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Leak-guard alerts</span>
              <span style={{ flex: 1 }} />
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>near-leaks &amp; anomalous outbound</span>
            </div>
            <div style={{ flex: 1 }}>
              {alerts.map((a, i) => (
                <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "var(--space-3)", padding: "var(--space-3) var(--space-4)",
                              borderBottom: i < alerts.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-base)", color: sevTone[a.sev], width: 20, textAlign: "center", lineHeight: 1.3 }}>{a.k}</span>
                  <div style={{ minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "baseline", gap: "var(--space-2)" }}>
                      <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1 }}>{a.title}</span>
                      <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{a.when}</span>
                    </div>
                    <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.45, marginTop: "var(--space-1)" }}>{a.note}</div>
                    <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-2)", alignItems: "center" }}>
                      <DojoChip tone={sevTone[a.sev]} soft={sevSoft[a.sev]}>{a.sev} severity</DojoChip>
                      <DojoChip tone={a.state === "quarantined" ? "var(--warning)" : "var(--ink-mute)"} soft={a.state === "quarantined" ? "var(--warning-soft)" : "var(--paper-mute)"}>{a.state}</DojoChip>
                      <DojoChip tone="var(--accent)" soft="var(--accent-soft)">客 {a.client}</DojoChip>
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)",
                          borderTop: "1px solid var(--paper-edge)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>守</span>
              <span style={{ flex: 1 }}>Shared into the client lead's incident view (Lead console)</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Members & roles ────────────────────────────────────── */
function DojoMembers({ mobile = false }) {
  const D = window.DOJO;
  const roleTone = { "Org admin": "var(--accent)", "Maintainer": "var(--ink)", "Contributor": "var(--ink-soft)", "Read-only": "var(--ink-faint)" };
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="任" eyebrow="Org · members" title="Members &amp; roles"
        sub="Roles are derived from git — the highest across a member's associated repos — then fine-tuned here."
        right={<div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">↻ Sync from git</DojoChip>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Bulk import · CSV · SCIM ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", cursor: "pointer" }}>Invite</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-3)", background: "var(--paper-soft)", border: "var(--hairline)",
                      borderLeft: "3px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", lineHeight: 1.2 }}>任</span>
          <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
            <b style={{ color: "var(--ink)", fontWeight: 600 }}>Just-in-time on first connect.</b> A new member is provisioned automatically at their git-derived role — but the auto-default is capped at <b style={{ fontWeight: 600 }}>Read-only</b>. Maintainer and Org admin are never granted automatically; they're elevated by hand below.
          </div>
        </div>
        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
          {!mobile && (
          <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: "var(--space-3)", padding: "var(--space-3) var(--space-4)",
                        borderBottom: "var(--hairline)", fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>
            <span>Member</span><span>Git role</span><span>Dōjō role</span><span>Scopes</span><span>Active</span>
          </div>
          )}
          {D.members.map((mm, i) => mobile ? (
            <div key={mm.name} style={{ padding: "var(--space-3) var(--space-4)", borderBottom: i < D.members.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <Avatar name={mm.name} size={28} />
                <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1, minWidth: 0 }}>{mm.name}</span>
                <button style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", background: "var(--paper)", border: "var(--hairline)",
                              borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-2)", cursor: "pointer", flexShrink: 0 }}>
                  <span style={{ fontSize: "var(--text-sm)", color: roleTone[mm.dojo] || "var(--ink)", fontWeight: 500 }}>{mm.dojo}</span>
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>▾</span>
                </button>
              </div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)", paddingLeft: 36 }}>{mm.git} · {mm.scopes} · {mm.active}</div>
            </div>
          ) : (
            <div key={mm.name} style={{ display: "grid", gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: "var(--space-3)", padding: "var(--space-3) var(--space-4)",
                          alignItems: "center", borderBottom: i < D.members.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <Avatar name={mm.name} size={28} />
                <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{mm.name}</span>
              </div>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{mm.git}</span>
              <button style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", background: "var(--paper)", border: "var(--hairline)",
                            borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-2)", cursor: "pointer", justifySelf: "start" }}>
                <span style={{ fontSize: "var(--text-sm)", color: roleTone[mm.dojo] || "var(--ink)", fontWeight: 500 }}>{mm.dojo}</span>
                <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>▾</span>
              </button>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{mm.scopes}</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{mm.active}</span>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-3)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, maxWidth: 760 }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>規</span>
          <span>Roles are derived from git then refined. Dōjō-only roles (admin · developer · tester) and per-project overrides — which git doesn't model — are set here; bulk onboarding runs through CSV or SCIM provisioning.</span>
        </div>
      </div>
    </div>
  );
}

/* ─── Scopes & policies ──────────────────────────────────── */
function DojoScopes({ mobile = false }) {
  const tree = [
    { k: "社", name: "Company", lvl: 0, preset: "Standard", share: "opt-in" },
    { k: "組", name: "Team · Payments", lvl: 1, preset: "Internal-only", share: "auto" },
    { k: "件", name: "Project · Ledger", lvl: 2, preset: "inherit", share: "inherit" },
    { k: "庫", name: "Repo · ledger-core", lvl: 3, preset: "inherit", share: "inherit" },
    { k: "技", name: "Stack · React", lvl: 1, preset: "Standard", share: "opt-in" },
    { k: "客", name: "Client · Globex", lvl: 1, client: true, preset: "Anonymized", share: "opt-in" },
  ];
  const ladder0 = [
    { k: "守", name: "Client anonymization", note: "drop client · repo · source before anything else applies", locked: true },
    { k: "庫", name: "Repo", note: "narrowest code scope" },
    { k: "件", name: "Project", note: "" },
    { k: "組", name: "Team", note: "" },
    { k: "社", name: "Company", note: "org-wide rules" },
    { k: "技", name: "Stack", note: "language / framework" },
    { k: "群", name: "Global · community", note: "shared upstream" },
    { k: "己", name: "Personal", note: "your own memory" },
  ];
  const [rungs, setRungs] = daS(ladder0);
  // The candidate under test: a client-origin guard bound to Repo ledger-core (in Team Payments).
  // Anonymization is a pinned pre-step; routing then goes to the topmost applicable *scope* rung.
  const APPLIES = ["Repo", "Project", "Team", "Company", "Stack"];
  const routed = rungs.find(r => !r.locked && APPLIES.includes(r.name));
  const move = (i, dir) => setRungs(rs => {
    const j = i + dir;
    if (j < 0 || j >= rs.length || rs[i].locked || rs[j].locked) return rs;
    const next = rs.slice(); [next[i], next[j]] = [next[j], next[i]]; return next;
  });
  const Panel = ({ title, note, children }) => (
    <window.DojoPanel title={title} note={note} align="baseline">{children}</window.DojoPanel>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="規" eyebrow="Org · policy" title="Scopes &amp; policies"
        sub="The scope hierarchy and its attribution / confidentiality rules. Templates seed a scope; the precedence ladder resolves conflicts top-down. Client-origin lessons are anonymized universally — client, repo and source dropped — before any scope rule applies."
        right={<div style={{ display: "flex", gap: "var(--space-2)" }}>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Apply template ▾</DojoChip>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)", display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(340px, 1fr))", gap: mobile ? "var(--space-4)" : "var(--space-5)", alignItems: "start" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-5)", minWidth: 0 }}>
          <Panel title="Scope hierarchy" note="company → team → project → repo · stack">
            <div style={{ display: "flex", flexDirection: "column" }}>
              {tree.map((s, i) => (
                <div key={s.name} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-2) var(--space-1)",
                              paddingLeft: `calc(var(--space-1) + ${s.lvl} * var(--space-5))`, borderBottom: i < tree.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-sm)", color: s.client ? "var(--accent)" : "var(--ink-mute)", width: 18, textAlign: "center" }}>{s.k}</span>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1 }}>{s.name}</span>
                  {s.client && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">anonymized</DojoChip>}
                  <DojoChip>{s.preset}</DojoChip>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", width: 54, textAlign: "right" }}>{s.share}</span>
                </div>
              ))}
            </div>
          </Panel>
          <Panel title="Test a candidate · which rule wins" note="simulate before you ship a policy">
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
                <DojoChip tone="var(--ink-soft)">type · guard ▾</DojoChip>
                <DojoChip tone="var(--ink-soft)">bound · Repo ledger-core ▾</DojoChip>
                <DojoChip tone="var(--accent)" soft="var(--accent-soft)">origin · Client Globex ▾</DojoChip>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-3)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>守</span>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.5 }}>
                  <b style={{ fontWeight: 600 }}>Client anonymization applies first</b> — client, repo and source are dropped. Then the <b style={{ fontWeight: 600, color: "var(--accent)" }}>{routed ? routed.name : "Company"}</b> rule routes it{routed && routed.note ? ` (${routed.note})` : ""}.
                </div>
              </div>
            </div>
          </Panel>
        </div>
        <Panel title="Precedence ladder" note="reorder · top wins">
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
            {rungs.map((r, i) => {
              const isRouted = routed && r.name === routed.name;
              return (
              <div key={r.name} style={{ display: "grid", gridTemplateColumns: "auto auto 1fr auto auto", gap: "var(--space-2)", alignItems: "center",
                            padding: "var(--space-2) var(--space-3)", borderRadius: "var(--radius-lg)",
                            background: r.locked ? "var(--accent-soft)" : isRouted ? "var(--paper-soft)" : "var(--paper)",
                            border: r.locked ? "1px solid var(--accent-edge)" : isRouted ? "1px solid var(--accent-edge)" : "var(--hairline)" }}>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", width: 12 }}>{i + 1}</span>
                <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
                  <button onClick={() => move(i, -1)} disabled={r.locked || i <= 1} title="Move up" style={{ lineHeight: 0.7, background: "none", border: "none", cursor: r.locked || i <= 1 ? "default" : "pointer", color: r.locked || i <= 1 ? "var(--ink-faint)" : "var(--ink-mute)", fontSize: "var(--text-xs)", padding: 0, opacity: r.locked ? 0.3 : 1 }}>▲</button>
                  <button onClick={() => move(i, 1)} disabled={r.locked || i >= rungs.length - 1} title="Move down" style={{ lineHeight: 0.7, background: "none", border: "none", cursor: r.locked || i >= rungs.length - 1 ? "default" : "pointer", color: r.locked || i >= rungs.length - 1 ? "var(--ink-faint)" : "var(--ink-mute)", fontSize: "var(--text-xs)", padding: 0, opacity: r.locked ? 0.3 : 1 }}>▼</button>
                </div>
                <div style={{ minWidth: 0, display: "flex", alignItems: "baseline", gap: "var(--space-2)", flexWrap: "wrap" }}>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{r.name}</span>
                  {r.note && <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{r.note}</span>}
                </div>
                {isRouted && <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>守</span>}
                {r.locked
                  ? <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", textTransform: "uppercase", letterSpacing: ".08em" }}>pinned</span>
                  : isRouted
                  ? <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", textTransform: "uppercase", letterSpacing: ".08em" }}>routes</span>
                  : <span style={{ width: 44 }} />}
              </div>
              );
            })}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── the admin console ──────────────────────────────────── */
function DojoAdminConsole({ initial = "overview", mobile = false, relayStart = null }) {
  const [active, setActive] = daS(initial);
  const go = (s) => { if (ADMIN_SECTIONS.includes(s)) setActive(s); };
  let screen;
  if (active === "monitor") screen = <DojoMonitor go={go} mobile={mobile} />;
  else if (active === "members") screen = <DojoMembers mobile={mobile} />;
  else if (active === "scopes") screen = <DojoScopes mobile={mobile} />;
  else if (active === "governance") screen = <DojoGovernance mobile={mobile} />;
  else if (active === "billing") screen = <DojoBilling mobile={mobile} />;
  else if (active === "identity") screen = <DojoIdentity mobile={mobile} />;
  else screen = <DojoOverview go={go} mobile={mobile} />;
  return (
    <DojoRoleShell label="Dōjō · Admin console" role={{ kanji: "長", label: "Org admin" }}
      nav={ADMIN_NAV} active={active} setActive={setActive} mobile={mobile} relayStart={relayStart}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoAdminConsole, DojoOverview, DojoMonitor, DojoMembers, DojoScopes });
