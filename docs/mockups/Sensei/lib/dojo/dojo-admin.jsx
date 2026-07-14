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
  { group: "Org", items: [
    { id: "members", kanji: "任", label: "Members & roles" },
    { id: "scopes",  kanji: "規", label: "Scopes & policies" },
  ]},
];
const ADMIN_SECTIONS = ["overview", "monitor", "members", "scopes"];

/* ─── Overview ───────────────────────────────────────────── */
function DojoOverview({ go }) {
  const D = window.DOJO, m = D.metrics;
  const published = [
    { kanji: "守", title: "Never log refresh tokens", scope: "Company", adoption: 0.92, delta: 6, status: "active" },
    { kanji: "紋", title: "Idempotency key on money-moving mutations", scope: "Team · Payments", adoption: 0.78, delta: 9, status: "active" },
    { kanji: "問", title: "Prefer optimistic UI for list mutations", scope: "Stack · React", adoption: 0.41, delta: -3, status: "flagged" },
  ];
  const Metric = ({ kanji, label, value, sub, children, onClick }) => (
    <div onClick={onClick} style={{ flex: 1, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12,
      padding: "16px 18px", cursor: onClick ? "pointer" : "default", minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 10 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)" }}>{label}</span>
      </div>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: 10 }}>
        <div className="display" style={{ fontSize: 34, fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{value}</div>
        {children}
      </div>
      {sub && <div style={{ fontSize: 11, color: "var(--ink-3)", marginTop: 8 }}>{sub}</div>}
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="全" eyebrow="Acme Corp · Dōjō" title="The shared mind, governed."
        sub="What your org has learned — triaged, approved, and routed to the scopes that need it."
        right={<div style={{ textAlign: "right", fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div>{D.org.scopes} scopes · {D.org.repos} repos</div>
          <div style={{ color: "var(--success)" }}>{m.incidents} confidentiality incidents</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        <div style={{ display: "flex", gap: 14 }}>
          <Metric kanji="門" label="Pending triage" value={m.pendingTriage} sub="across 4 scopes · oldest 3d">
            <span className="mono" style={{ fontSize: 11, color: "var(--accent)" }}>maintainers →</span>
          </Metric>
          <Metric kanji="共" label="Contributions · 7d" value={m.contribWeek}>
            <Sparkline data={m.contribSpark} width={92} height={30} color="var(--accent)" fill="var(--accent-soft)" />
          </Metric>
          <Metric kanji="決" label="Approved · 7d" value={m.approvedWeek} sub="published to matching scopes" />
          <Metric kanji="果" label="Adoption lift" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} sub="FTR across adopting scopes">
            <Sparkline data={m.ftrSpark} width={92} height={30} color="var(--success)" />
          </Metric>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1fr", gap: 18, marginTop: 18 }}>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", padding: "13px 16px", borderBottom: "var(--hairline)" }}>
              <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>Top of the triage queue</span>
              <span style={{ flex: 1 }} />
              <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>maintainers own review</span>
            </div>
            {D.queue.slice(0, 4).map((c, i) => (
              <div key={c.id} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 12, alignItems: "center",
                width: "100%", textAlign: "left", padding: "12px 16px", borderBottom: i < 3 ? "1px solid var(--edge)" : "none" }}>
                <span className="kanji" style={{ fontSize: 17, color: "var(--accent)", width: 20, textAlign: "center" }}>{c.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.title}</div>
                  <div style={{ display: "flex", gap: 7, marginTop: 4, alignItems: "center" }}>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-3)" }}>{c.scope}</span>
                    <OriginChip origin={c.origin} />
                  </div>
                </div>
                <Confidence v={c.confidence} w={56} />
              </div>
            ))}
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
            <div style={{ background: "var(--paper-2)", border: "1px solid var(--success-edge)", borderRadius: 12, padding: "15px 17px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <span className="kanji" style={{ fontSize: 15, color: "var(--success)" }}>盾</span>
                <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>Confidentiality</span>
              </div>
              <div style={{ fontSize: 13, color: "var(--ink)", lineHeight: 1.55 }}>
                <b style={{ fontWeight: 600 }}>{m.dereferenced}</b> client lessons auto-dereferenced this week ·
                <span style={{ color: "var(--success)" }}> 0 incidents</span>. Sources dropped automatically; only flagged exceptions reach a lead.
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 7, marginTop: 10, paddingTop: 10, borderTop: "1px solid var(--edge)" }}>
                <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success)" }} />
                <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: ".06em" }}>Leak-guard armed</span>
                <span style={{ flex: 1 }} />
                <span style={{ fontSize: 11, color: "var(--ink-3)" }}>alerts shared to the client lead <span style={{ color: "var(--accent)" }}>→</span></span>
              </div>
            </div>
            <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden", flex: 1 }}>
              <div style={{ padding: "13px 16px", borderBottom: "var(--hairline)", fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>Recent activity</div>
              <div style={{ padding: "4px 0" }}>
                {D.activity.map((a, i) => (
                  <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 10, alignItems: "start", padding: "9px 16px" }}>
                    <span className="kanji" style={{ fontSize: 13, width: 16, textAlign: "center",
                                  color: a.tone === "success" ? "var(--success)" : a.tone === "accent" ? "var(--accent)" : "var(--ink-3)" }}>{a.kanji}</span>
                    <span style={{ fontSize: 12, color: "var(--ink-2)", lineHeight: 1.45 }}>{a.text}</span>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{a.when}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
        <div style={{ marginTop: 18 }}>
          <div style={{ display: "flex", alignItems: "center", marginBottom: 9 }}>
            <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Published · adoption &amp; health</span>
            <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", marginLeft: 10 }}>the Impact loop, scoped to the org</span>
          </div>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
            {published.map((p, i) => {
              const neg = p.delta < 0;
              return (
              <div key={p.title} style={{ display: "grid", gridTemplateColumns: "auto 1fr 150px 92px 168px", gap: 14, alignItems: "center",
                            padding: "13px 16px", borderBottom: i < published.length - 1 ? "1px solid var(--edge)" : "none",
                            background: neg ? "var(--warning-soft)" : "transparent" }}>
                <span className="kanji" style={{ fontSize: 17, color: neg ? "oklch(0.52 0.13 60)" : "var(--accent)", width: 20, textAlign: "center" }}>{p.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.title}</div>
                  <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)", marginTop: 3 }}>{p.scope}</div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <div style={{ flex: 1, height: 4, borderRadius: 2, background: "var(--paper-3)", overflow: "hidden" }}>
                    <div style={{ width: (p.adoption * 100) + "%", height: "100%", background: "var(--accent)", borderRadius: 2 }} />
                  </div>
                  <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{Math.round(p.adoption * 100)}%</span>
                </div>
                <span className="mono" style={{ fontSize: 12, color: neg ? "oklch(0.52 0.13 60)" : "var(--success)", textAlign: "right" }}>
                  {neg ? "" : "+"}{p.delta}pp FTR
                </span>
                <div style={{ display: "flex", justifyContent: "flex-end" }}>
                  {neg
                    ? <button style={{ display: "inline-flex", alignItems: "center", gap: 6, padding: "6px 12px", borderRadius: 7,
                              border: "1px solid oklch(0.52 0.13 60/.4)", background: "var(--paper)", color: "oklch(0.5 0.13 60)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}>
                        <span className="kanji" style={{ fontSize: 12 }}>退</span> Retract downstream
                      </button>
                    : <DojoChip tone="var(--success)" soft="var(--success-soft)">active</DojoChip>}
                </div>
              </div>
              );
            })}
            <div style={{ display: "flex", alignItems: "center", gap: 7, padding: "10px 16px", borderTop: "1px solid var(--edge)", fontSize: 11.5, color: "var(--ink-3)", lineHeight: 1.45 }}>
              <span className="kanji" style={{ fontSize: 12, color: "oklch(0.52 0.13 60)" }}>退</span>
              <span>Lifecycle <b style={{ fontWeight: 600, color: "var(--ink-2)" }}>active → deprecated → retracted</b>. Negative impact is flagged automatically; one-click retract pulls a teaching back and notifies adopters.</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Monitor · hive-mind health ─────────────────────────── */
function DojoMonitor({ go }) {
  const D = window.DOJO, m = D.metrics;
  const days = ["M", "T", "W", "T", "F", "S", "S"];
  const weeks = m.contribSpark.map((c, i) => ({ c, a: Math.max(0, Math.round(c * (0.62 + i * 0.025))) }));
  const maxBar = Math.max(...weeks.map(w => w.c));
  const approvalRate = Math.round((m.approvedWeek / m.contribWeek) * 100);
  const sevTone = { high: "oklch(0.55 0.17 25)", med: "oklch(0.52 0.13 60)", low: "var(--ink-3)" };
  const sevSoft = { high: "oklch(0.93 0.04 25)", med: "var(--warning-soft)", low: "var(--paper-3)" };
  const alerts = [
    { sev: "high", k: "警", title: "Anomalous outbound volume — Initech scope", state: "quarantined", client: "Initech", when: "2h",
      note: "14 lessons queued outbound in 5 min; auto-quarantined and held pending a lead review." },
    { sev: "med", k: "盾", title: "Named entity survived dereference — Globex", state: "exception", client: "Globex", when: "5h",
      note: "A tenant-id naming scheme cleared the classifier; routed to the client lead's exception queue." },
    { sev: "low", k: "盾", title: "Rare-context lesson flagged — Globex", state: "exception", client: "Globex", when: "1d",
      note: "Low k-anonymity; held for confirmation rather than weakened." },
  ];
  const Signal = ({ kanji, label, value, sub, tone = "var(--ink)", children }) => (
    <div style={{ flex: 1, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px", minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)" }}>{label}</span>
      </div>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: 10 }}>
        <div className="display" style={{ fontSize: 36, fontWeight: 300, lineHeight: 1, color: tone }}>{value}</div>
        {children}
      </div>
      <div style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 9, lineHeight: 1.4 }}>{sub}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="観" eyebrow="Org · monitor" title="Hive-mind health"
        sub="Three headline signals — throughput, adoption, and leak-guard — read over the full audit trail. Anomalies surface here and flow straight into the client lead's incident view."
        right={<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Last 7 days ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, fontFamily: "var(--font-mono)",
                        color: "var(--success)", background: "var(--success-soft)", border: "1px solid var(--success-edge)",
                        borderRadius: 20, padding: "3px 10px" }}>● leak-guard armed</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        <div style={{ display: "flex", gap: 14 }}>
          <Signal kanji="通" label="Throughput · 7d" value={`${m.approvedWeek}/${m.contribWeek}`}
            sub={`${approvalRate}% approved · ${m.contribWeek} contributed, ${m.approvedWeek} published`}>
            <Sparkline data={m.contribSpark} width={96} height={32} color="var(--accent)" fill="var(--accent-soft)" />
          </Signal>
          <Signal kanji="果" label="Adoption rate" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} tone="var(--success)"
            sub="FTR lift across adopting scopes, trending up week over week">
            <Sparkline data={m.ftrSpark} width={96} height={32} color="var(--success)" />
          </Signal>
          <Signal kanji="盾" label="Leak-guard · 7d" value={alerts.length}
            sub={`${m.incidents} confidentiality incidents · ${m.dereferenced} sources dropped`}>
            <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 4 }}>
              <div style={{ display: "flex", gap: 4 }}>
                {alerts.map((a, i) => <span key={i} style={{ width: 9, height: 9, borderRadius: "50%", background: sevTone[a.sev] }} />)}
              </div>
              <span className="mono" style={{ fontSize: 9.5, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".06em" }}>2 exceptions · 1 held</span>
            </div>
          </Signal>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1.35fr 1fr", gap: 18, marginTop: 18 }}>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 14, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
              <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Contributions vs. approvals</span>
              <span style={{ flex: 1 }} />
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-3)" }}><span style={{ width: 9, height: 9, borderRadius: 2, background: "var(--accent)", opacity: .85 }} /> contributed</span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-3)" }}><span style={{ width: 9, height: 9, borderRadius: 2, background: "var(--success)" }} /> approved</span>
            </div>
            <div style={{ padding: "20px 18px 16px" }}>
              <div style={{ display: "flex", alignItems: "flex-end", gap: 16, height: 122 }}>
                {weeks.map((w, i) => (
                  <div key={i} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 7 }}>
                    <div style={{ height: 96, width: "100%", display: "flex", alignItems: "flex-end", justifyContent: "center", gap: 5 }}>
                      <div style={{ width: 11, height: Math.round(w.c / maxBar * 96), background: "var(--accent)", opacity: .85, borderRadius: "3px 3px 0 0" }} />
                      <div style={{ width: 11, height: Math.round(w.a / maxBar * 96), background: "var(--success)", borderRadius: "3px 3px 0 0" }} />
                    </div>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{days[i]}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden", display: "flex", flexDirection: "column" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
              <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>警</span>
              <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Leak-guard alerts</span>
              <span style={{ flex: 1 }} />
              <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>near-leaks &amp; anomalous outbound</span>
            </div>
            <div style={{ flex: 1 }}>
              {alerts.map((a, i) => (
                <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: 11, padding: "12px 16px",
                              borderBottom: i < alerts.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji" style={{ fontSize: 16, color: sevTone[a.sev], width: 20, textAlign: "center", lineHeight: 1.3 }}>{a.k}</span>
                  <div style={{ minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                      <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}>{a.title}</span>
                      <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{a.when}</span>
                    </div>
                    <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.45, marginTop: 3 }}>{a.note}</div>
                    <div style={{ display: "flex", gap: 7, marginTop: 7, alignItems: "center" }}>
                      <DojoChip tone={sevTone[a.sev]} soft={sevSoft[a.sev]}>{a.sev} severity</DojoChip>
                      <DojoChip tone={a.state === "quarantined" ? "oklch(0.52 0.13 60)" : "var(--ink-3)"} soft={a.state === "quarantined" ? "var(--warning-soft)" : "var(--paper-3)"}>{a.state}</DojoChip>
                      <DojoChip tone="var(--accent)" soft="var(--accent-soft)">客 {a.client}</DojoChip>
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "11px 16px",
                          borderTop: "1px solid var(--edge)", fontSize: 11.5, color: "var(--ink-3)" }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>守</span>
              <span style={{ flex: 1 }}>Shared into the client lead's incident view (Lead console)</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Members & roles ────────────────────────────────────── */
function DojoMembers() {
  const D = window.DOJO;
  const roleTone = { "Org admin": "var(--accent)", "Maintainer": "var(--ink)", "Contributor": "var(--ink-2)", "Read-only": "var(--ink-4)" };
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="任" eyebrow="Org · members" title="Members &amp; roles"
        sub="Roles are derived from git — the highest across a member's associated repos — then fine-tuned here."
        right={<div style={{ display: "flex", gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">↻ Sync from git</DojoChip>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Bulk import · CSV · SCIM ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", padding: "6px 12px", borderRadius: 7, background: "var(--ink)", color: "var(--paper)", fontSize: 12.5, cursor: "pointer" }}>Invite</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 11, background: "var(--paper-2)", border: "var(--hairline)",
                      borderLeft: "3px solid var(--accent)", borderRadius: 10, padding: "13px 16px", marginBottom: 18 }}>
          <span className="kanji" style={{ fontSize: 16, color: "var(--accent)", lineHeight: 1.2 }}>任</span>
          <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
            <b style={{ color: "var(--ink)", fontWeight: 600 }}>Just-in-time on first connect.</b> A new member is provisioned automatically at their git-derived role — but the auto-default is capped at <b style={{ fontWeight: 600 }}>Read-only</b>. Maintainer and Org admin are never granted automatically; they're elevated by hand below.
          </div>
        </div>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
          <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: 14, padding: "11px 18px",
                        borderBottom: "var(--hairline)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>
            <span>Member</span><span>Git role</span><span>Dōjō role</span><span>Scopes</span><span>Active</span>
          </div>
          {D.members.map((mm, i) => (
            <div key={mm.name} style={{ display: "grid", gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: 14, padding: "13px 18px",
                          alignItems: "center", borderBottom: i < D.members.length - 1 ? "1px solid var(--edge)" : "none" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <Avatar name={mm.name} size={28} />
                <span style={{ fontSize: 13, color: "var(--ink)" }}>{mm.name}</span>
              </div>
              <span className="mono" style={{ fontSize: 11.5, color: "var(--ink-3)" }}>{mm.git}</span>
              <button style={{ display: "inline-flex", alignItems: "center", gap: 6, background: "var(--paper)", border: "var(--hairline)",
                            borderRadius: 6, padding: "4px 9px", cursor: "pointer", justifySelf: "start" }}>
                <span style={{ fontSize: 12.5, color: roleTone[mm.dojo] || "var(--ink)", fontWeight: 500 }}>{mm.dojo}</span>
                <span style={{ fontSize: 8, color: "var(--ink-4)" }}>▾</span>
              </button>
              <span style={{ fontSize: 12, color: "var(--ink-2)" }}>{mm.scopes}</span>
              <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{mm.active}</span>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 9, marginTop: 14, fontSize: 12, color: "var(--ink-3)", lineHeight: 1.5, maxWidth: 760 }}>
          <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>規</span>
          <span>Roles are derived from git then refined. Dōjō-only roles (admin · developer · tester) and per-project overrides — which git doesn't model — are set here; bulk onboarding runs through CSV or SCIM provisioning.</span>
        </div>
      </div>
    </div>
  );
}

/* ─── Scopes & policies ──────────────────────────────────── */
function DojoScopes() {
  const tree = [
    { k: "社", name: "Company", lvl: 0, preset: "Standard", share: "opt-in" },
    { k: "組", name: "Team · Payments", lvl: 1, preset: "Internal-only", share: "auto" },
    { k: "件", name: "Project · Ledger", lvl: 2, preset: "inherit", share: "inherit" },
    { k: "庫", name: "Repo · ledger-core", lvl: 3, preset: "inherit", share: "inherit" },
    { k: "技", name: "Stack · React", lvl: 1, preset: "Standard", share: "opt-in" },
    { k: "客", name: "Client · Globex", lvl: 1, client: true, preset: "Anonymized", share: "opt-in" },
  ];
  const ladder = [
    { k: "守", name: "Client anonymization", note: "strip client · repo · source before anything else applies", locked: true },
    { k: "庫", name: "Repo", note: "narrowest code scope" },
    { k: "件", name: "Project", note: "" },
    { k: "組", name: "Team", note: "" },
    { k: "社", name: "Company", note: "org-wide rules" },
    { k: "技", name: "Stack", note: "language / framework" },
    { k: "群", name: "Global · community", note: "shared upstream" },
    { k: "己", name: "Personal", note: "your own memory" },
  ];
  const Panel = ({ title, note, children }) => (
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
      </div>
      <div style={{ padding: 14 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="規" eyebrow="Org · policy" title="Scopes &amp; policies"
        sub="The scope hierarchy and its attribution / confidentiality rules. Templates seed a scope; the precedence ladder resolves conflicts top-down. Client-origin lessons are anonymized universally — client, repo and source dropped — before any scope rule applies."
        right={<div style={{ display: "flex", gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Apply template ▾</DojoChip>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "grid", gridTemplateColumns: "minmax(0,1fr) 392px", gap: 22, alignItems: "start" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 22, minWidth: 0 }}>
          <Panel title="Scope hierarchy" note="company → team → project → repo · stack">
            <div style={{ display: "flex", flexDirection: "column" }}>
              {tree.map((s, i) => (
                <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 10, padding: "9px 6px",
                              paddingLeft: 6 + s.lvl * 22, borderBottom: i < tree.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji" style={{ fontSize: 14, color: s.client ? "var(--accent)" : "var(--ink-3)", width: 18, textAlign: "center" }}>{s.k}</span>
                  <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}>{s.name}</span>
                  {s.client && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">source dropped</DojoChip>}
                  <DojoChip>{s.preset}</DojoChip>
                  <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", width: 54, textAlign: "right" }}>{s.share}</span>
                </div>
              ))}
            </div>
          </Panel>
          <Panel title="Test a candidate · which rule wins" note="simulate before you ship a policy">
            <div style={{ display: "flex", flexDirection: "column", gap: 11 }}>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <DojoChip tone="var(--ink-2)">type · guard ▾</DojoChip>
                <DojoChip tone="var(--ink-2)">bound · Repo ledger-core ▾</DojoChip>
                <DojoChip tone="var(--accent)" soft="var(--accent-soft)">origin · Client Globex ▾</DojoChip>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 11, background: "var(--paper)", border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 9, padding: "11px 14px" }}>
                <span className="kanji" style={{ fontSize: 16, color: "var(--accent)" }}>守</span>
                <div style={{ fontSize: 12.5, color: "var(--ink)", lineHeight: 1.5 }}>
                  <b style={{ fontWeight: 600 }}>Client anonymization wins.</b> The lesson is anonymized — client, repo and source dropped — before any repo or team rule applies, then routes by its binding.
                </div>
              </div>
            </div>
          </Panel>
        </div>
        <Panel title="Precedence ladder" note="drag to reorder · top wins">
          <div style={{ display: "flex", flexDirection: "column", gap: 7 }}>
            {ladder.map((r, i) => (
              <div key={r.name} style={{ display: "grid", gridTemplateColumns: "auto auto 1fr auto", gap: 10, alignItems: "center",
                            padding: "9px 11px", borderRadius: 8,
                            background: r.locked ? "var(--accent-soft)" : "var(--paper)",
                            border: r.locked ? "1px solid oklch(0.58 0.15 35/.28)" : "var(--hairline)" }}>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", width: 12 }}>{i + 1}</span>
                <span style={{ fontSize: 13, color: "var(--ink-4)", opacity: r.locked ? 0.4 : 1, cursor: r.locked ? "default" : "grab", letterSpacing: "-2px" }}>⠿</span>
                <div style={{ minWidth: 0, display: "flex", alignItems: "baseline", gap: 8, flexWrap: "wrap" }}>
                  <span style={{ fontSize: 13, color: "var(--ink)" }}>{r.name}</span>
                  {r.note && <span style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{r.note}</span>}
                </div>
                {r.locked && <span className="mono" style={{ fontSize: 9, color: "var(--accent)", textTransform: "uppercase", letterSpacing: ".08em" }}>pinned</span>}
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── the admin console ──────────────────────────────────── */
function DojoAdminConsole({ initial = "overview" }) {
  const [active, setActive] = daS(initial);
  const go = (s) => { if (ADMIN_SECTIONS.includes(s)) setActive(s); };
  let screen;
  if (active === "monitor") screen = <DojoMonitor go={go} />;
  else if (active === "members") screen = <DojoMembers />;
  else if (active === "scopes") screen = <DojoScopes />;
  else screen = <DojoOverview go={go} />;
  return (
    <DojoRoleShell label="Dōjō · Admin console" role={{ kanji: "長", label: "Org admin" }}
      nav={ADMIN_NAV} active={active} setActive={setActive}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoAdminConsole, DojoOverview, DojoMonitor, DojoMembers, DojoScopes });
