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
  { group: "Org · manage", items: [
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
    <div className="flex-1 bg-paper-soft border border-paper-edge rounded-lg py-4 px-4 min-w-0" onClick={onClick} style={{ cursor: onClick ? "pointer" : "default" }}>
      <div className="flex items-center gap-2 mb-2" >
        <span className="kanji text-sm text-accent" >{kanji}</span>
        <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".12em" }}>{label}</span>
      </div>
      <div className="flex items-end justify-between gap-2" >
        <div className="display text-2xl font-light text-ink" style={{ lineHeight: 1 }}>{value}</div>
        {children}
      </div>
      {sub && <div className="text-xs text-ink-mute mt-2" >{sub}</div>}
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper relative" >
      <DojoHead mobile={mobile} kanji="全" eyebrow="Acme Corp · Dōjō" title="The shared mind, governed."
        sub="What your org has learned — triaged, approved, and routed to the scopes that need it."
        right={<div className="text-right text-xs text-ink-mute" style={{ fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div>{D.org.scopes} scopes · {D.org.repos} repos</div>
          <div className="text-success" >{m.incidents} confidentiality incidents</div>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="flex gap-3" style={{ flexDirection: mobile ? "column" : "row" }}>
          <Metric kanji="門" label="Pending triage" value={m.pendingTriage} sub="across 4 scopes · oldest 3d">
            <span className="mono text-xs text-accent" >maintainers →</span>
          </Metric>
          <Metric kanji="共" label="Contributions · 7d" value={m.contribWeek}>
            <Sparkline data={m.contribSpark} width={92} height={30} color="var(--accent)" fill="var(--accent-soft)" />
          </Metric>
          <Metric kanji="決" label="Approved · 7d" value={m.approvedWeek} sub="published to matching scopes" />
          <Metric kanji="果" label="Adoption lift" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} sub="FTR across adopting scopes">
            <Sparkline data={m.ftrSpark} width={92} height={30} color="var(--success)" />
          </Metric>
        </div>
        <div className="grid gap-4 mt-4" style={{ gridTemplateColumns: mobile ? "1fr" : "1.4fr 1fr" }}>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            <div className="flex items-center py-3 px-4 border-b" >
              <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".14em" }}>Top of the triage queue</span>
              <span className="flex-1" />
              <span className="mono text-xs text-ink-faint" >maintainers own review</span>
            </div>
            {D.queue.slice(0, 4).map((c, i) => (
              <div className="grid gap-3 items-center w-full text-left py-3 px-4" key={c.id} style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: i < 3 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="kanji text-lg text-accent text-center" style={{ width: 20 }}>{c.kanji}</span>
                <div className="min-w-0" >
                  <div className="text-sm text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{c.title}</div>
                  <div className="flex gap-2 mt-1 items-center" >
                    <span className="mono text-xs text-ink-mute" >{c.scope}</span>
                    <OriginChip origin={c.origin} />
                  </div>
                </div>
                <Confidence v={c.confidence} w={56} />
              </div>
            ))}
          </div>
          <div className="flex flex-col gap-4" >
            <div className="bg-paper-soft rounded-lg py-4 px-4" style={{ border: "1px solid var(--success-edge)" }}>
              <div className="flex items-center gap-2 mb-2" >
                <span className="kanji text-base text-success" >盾</span>
                <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".14em" }}>Confidentiality</span>
              </div>
              <div className="text-sm text-ink" style={{ lineHeight: 1.55 }}>
                <b className="font-semibold" >{m.anonymized}</b> client lessons auto-anonymized this week ·
                <span className="text-success" > 0 incidents</span>. Sources dropped automatically; only flagged exceptions reach a lead.
              </div>
              <div className="flex items-center gap-2 mt-2 pt-2" style={{ borderTop: "1px solid var(--paper-edge)" }}>
                <span className="rounded-full bg-success" style={{ width: 6, height: 6 }} />
                <span className="mono text-xs text-ink-mute uppercase" style={{ letterSpacing: ".06em" }}>Leak-guard armed</span>
                <span className="flex-1" />
                <span className="text-xs text-ink-mute" >alerts shared to the client lead <span className="text-accent" >→</span></span>
              </div>
            </div>
            <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden flex-1" >
              <div className="py-3 px-4 border-b text-xs uppercase text-ink-mute" style={{ letterSpacing: ".14em" }}>Recent activity</div>
              <div className="py-1 px-0" >
                {D.activity.map((a, i) => (
                  <div className="grid gap-2 items-start py-2 px-4" key={i} style={{ gridTemplateColumns: "auto 1fr auto" }}>
                    <span className="kanji text-sm text-center" style={{ width: 16,
 color: a.tone === "success" ? "var(--success)" : a.tone === "accent" ? "var(--accent)" : "var(--ink-mute)" }}>{a.kanji}</span>
                    <span className="text-xs text-ink-soft" style={{ lineHeight: 1.45 }}>{a.text}</span>
                    <span className="mono text-xs text-ink-faint" >{a.when}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
        <div className="mt-4" >
          <div className="flex items-center mb-2" >
            <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Published · adoption &amp; health</span>
            <span className="mono text-xs text-ink-faint ml-2" >the Impact loop, scoped to the org</span>
          </div>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" style={{ overflowX: mobile ? "auto" : "hidden" }}>
            {published.map((p, i) => {
              const neg = p.delta < 0;
              return (
              <div className="grid gap-3 items-center py-3 px-4" key={p.title} style={{ gridTemplateColumns: "auto 1fr 150px 92px 168px", borderBottom: i < published.length - 1 ? "1px solid var(--paper-edge)" : "none",
 background: neg ? "var(--warning-soft)" : "transparent" }}>
                <span className="kanji text-lg text-center" style={{ color: neg ? "var(--warning)" : "var(--accent)", width: 20 }}>{p.kanji}</span>
                <div className="min-w-0" >
                  <div className="text-sm text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{p.title}</div>
                  <div className="mono text-xs text-ink-faint mt-1" >{p.scope}</div>
                </div>
                <div className="flex items-center gap-2" >
                  <div className="flex-1 rounded-sm bg-paper-mute overflow-hidden" style={{ height: 4 }}>
                    <div className="h-full bg-accent rounded-sm" style={{ width: (p.adoption * 100) + "%" }} />
                  </div>
                  <span className="mono text-xs text-ink-mute" >{Math.round(p.adoption * 100)}%</span>
                </div>
                <span className="mono text-xs text-right" style={{ color: neg ? "var(--warning)" : "var(--success)" }}>
                  {neg ? "" : "+"}{p.delta}pp FTR
                </span>
                <div className="flex justify-end" >
                  {retracted.includes(p.title)
                    ? <DojoChip tone="var(--danger)" soft="var(--danger-soft)">退 retracted</DojoChip>
                    : neg
                    ? <button className="inline-flex items-center gap-1 py-1 px-3 rounded bg-paper text-danger text-xs cursor-pointer" onClick={() => setRetract(p)} style={{
 border: "1px solid var(--danger-edge)", fontFamily: "inherit" }}>
                        <span className="kanji text-xs" >退</span> Retract downstream
                      </button>
                    : <DojoChip tone="var(--success)" soft="var(--success-soft)">active</DojoChip>}
                </div>
              </div>
              );
            })}
            <div className="flex items-center gap-2 py-2 px-4 text-xs text-ink-mute" style={{ borderTop: "1px solid var(--paper-edge)", lineHeight: 1.45 }}>
              <span className="kanji text-xs text-warning" >退</span>
              <span>Lifecycle <b className="font-semibold text-ink-soft" >active → deprecated → retracted</b>. Negative impact is flagged automatically; one-click retract pulls a teaching back and notifies adopters.</span>
            </div>
          </div>
        </div>
      </div>
      {retract && (
        <div className="absolute flex items-center justify-center p-6" onClick={() => setRetract(null)} style={{ inset: 0, zIndex: 60, background: "var(--scrim)" }}>
          <div className="max-w-full bg-paper rounded-lg shadow-lg overflow-hidden" onClick={e => e.stopPropagation()} style={{ width: 480, border: "1px solid var(--danger-edge)" }}>
            <div className="flex items-center gap-2 p-4 border-b" >
              <span className="kanji text-xl text-danger" >退</span>
              <div>
                <div className="text-xs uppercase text-danger" style={{ letterSpacing: ".14em", fontWeight: 700 }}>Retract downstream</div>
                <div className="text-sm text-ink mt-1" >{retract.title}</div>
              </div>
            </div>
            <div className="p-4" >
              <div className="text-sm text-ink-soft mb-3" style={{ lineHeight: 1.55 }}>
                This teaching is showing <b className="font-semibold text-warning" >{retract.delta}pp FTR</b> in <b className="font-semibold text-ink" >{retract.scope}</b>. Retracting pulls it from every adopting scope and notifies adopters — the lesson moves to <b className="font-semibold" >retracted</b> in the audit trail.
              </div>
              <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden mb-4" >
                <div className="py-2 px-3 border-b text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".1em" }}>What this affects</div>
                {[["場", "Adopting scopes", "Company · 3 teams · 11 repos"], ["人", "Adopters notified", "24 contributors — with the reason"], ["録", "Audit", "active → deprecated → retracted, hash-chained"]].map(([k, l, v]) => (
                  <div className="grid gap-2 items-center py-2 px-3" key={l} style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: "1px solid var(--paper-edge)" }}>
                    <span className="kanji text-sm text-ink-mute text-center" style={{ width: 16 }}>{k}</span>
                    <span className="text-sm text-ink" >{l}</span>
                    <span className="mono text-xs text-ink-faint text-right" >{v}</span>
                  </div>
                ))}
              </div>
              <div className="flex justify-end gap-2" >
                <button className="py-2 px-4 rounded-lg border border-paper-edge bg-paper text-ink-soft text-sm cursor-pointer" onClick={() => setRetract(null)} style={{ fontFamily: "inherit" }}>Cancel</button>
                <button className="inline-flex items-center gap-1 py-2 px-4 rounded-lg border-0 bg-danger text-paper text-sm font-medium cursor-pointer" onClick={() => { setRetracted(a => [...a, retract.title]); setRetract(null); }} style={{ fontFamily: "inherit" }}>
                  <span className="kanji text-sm text-paper" >退</span> Retract &amp; notify
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/* ─── Monitor · Dōjō health ───────────────────────────────── */
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
    <div className="flex-1 bg-paper-soft border border-paper-edge rounded-lg py-4 px-4 min-w-0" >
      <div className="flex items-center gap-2 mb-3" >
        <span className="kanji text-sm text-accent" >{kanji}</span>
        <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".12em" }}>{label}</span>
      </div>
      <div className="flex items-end justify-between gap-2" >
        <div className="display text-3xl font-light" style={{ lineHeight: 1, color: tone }}>{value}</div>
        {children}
      </div>
      <div className="text-xs text-ink-mute mt-2" style={{ lineHeight: 1.4 }}>{sub}</div>
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper relative" >
      <DojoHead mobile={mobile} kanji="観" eyebrow="Org · monitor" title="Dōjō health"
        sub="Three headline signals — throughput, adoption, and leak-guard — read over the full audit trail. Anomalies surface here and flow straight into the client lead's incident view."
        right={<div className="flex gap-2 items-center" >
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Last 7 days ▾</DojoChip>
          <span className="inline-flex items-center gap-1 text-xs text-success bg-success-soft rounded-full py-1 px-2" style={{ fontFamily: "var(--font-mono)", border: "1px solid var(--success-edge)" }}>● leak-guard armed</span>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="flex gap-3" style={{ flexDirection: mobile ? "column" : "row" }}>
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
            <div className="flex flex-col items-end gap-1" >
              <div className="flex gap-1" >
                {alerts.map((a, i) => <span className="rounded-full" key={i} style={{ width: 9, height: 9, background: sevTone[a.sev] }} />)}
              </div>
              <span className="mono text-xs text-ink-faint uppercase" style={{ letterSpacing: ".06em" }}>2 exceptions · 1 held</span>
            </div>
          </Signal>
        </div>
        <div className="grid gap-4 mt-4" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(340px, 1fr))" }}>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            <div className="flex items-center gap-3 py-3 px-4 border-b flex-wrap" >
              <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Contributions vs. approvals</span>
              <span className="flex-1" />
              <span className="inline-flex items-center gap-1 text-xs text-ink-mute" ><span className="rounded-sm bg-accent" style={{ width: 9, height: 9, opacity: .85 }} /> contributed</span>
              <span className="inline-flex items-center gap-1 text-xs text-ink-mute" ><span className="rounded-sm bg-success" style={{ width: 9, height: 9 }} /> approved</span>
            </div>
            <div className="flex-1 p-4 flex flex-col justify-end" >
              <div className="flex items-end gap-4 h-full" style={{ minHeight: 110 }}>
                {weeks.map((w, i) => (
                  <div className="flex-1 flex flex-col items-center gap-2 h-full" key={i} >
                    <div className="flex-1 w-full flex items-end justify-center gap-1" >
                      <div className="bg-accent" style={{ width: 11, height: (w.c / maxBar * 100) + "%", opacity: .85, borderRadius: "var(--radius-sm) var(--radius-sm) 0 0" }} />
                      <div className="bg-success" style={{ width: 11, height: (w.a / maxBar * 100) + "%", borderRadius: "var(--radius-sm) var(--radius-sm) 0 0" }} />
                    </div>
                    <span className="mono text-xs text-ink-faint" >{days[i]}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden flex flex-col" >
            <div className="flex items-center gap-2 py-3 px-4 border-b" >
              <span className="kanji text-sm text-accent" >警</span>
              <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Leak-guard alerts</span>
              <span className="flex-1" />
              <span className="mono text-xs text-ink-faint" >near-leaks &amp; anomalous outbound</span>
            </div>
            <div className="flex-1" >
              {alerts.map((a, i) => (
                <div className="grid gap-3 py-3 px-4" key={i} style={{ gridTemplateColumns: "auto 1fr",
 borderBottom: i < alerts.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <span className="kanji text-base text-center" style={{ color: sevTone[a.sev], width: 20, lineHeight: 1.3 }}>{a.k}</span>
                  <div className="min-w-0" >
                    <div className="flex items-baseline gap-2" >
                      <span className="text-sm text-ink flex-1" >{a.title}</span>
                      <span className="mono text-xs text-ink-faint" >{a.when}</span>
                    </div>
                    <div className="text-xs text-ink-soft mt-1" style={{ lineHeight: 1.45 }}>{a.note}</div>
                    <div className="flex gap-2 mt-2 items-center" >
                      <DojoChip tone={sevTone[a.sev]} soft={sevSoft[a.sev]}>{a.sev} severity</DojoChip>
                      <DojoChip tone={a.state === "quarantined" ? "var(--warning)" : "var(--ink-mute)"} soft={a.state === "quarantined" ? "var(--warning-soft)" : "var(--paper-mute)"}>{a.state}</DojoChip>
                      <DojoChip tone="var(--accent)" soft="var(--accent-soft)">客 {a.client}</DojoChip>
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <div className="flex items-center gap-2 py-3 px-4 text-xs text-ink-mute" style={{
 borderTop: "1px solid var(--paper-edge)" }}>
              <span className="kanji text-sm text-accent" >守</span>
              <span className="flex-1" >Shared into the client lead's incident view (Lead console)</span>
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="任" eyebrow="Org · members" title="Members &amp; roles"
        sub="Roles are derived from git — the highest across a member's associated repos — then fine-tuned here."
        right={<div className="flex gap-2 flex-wrap" >
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">↻ Sync from git</DojoChip>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Bulk import · CSV · SCIM ▾</DojoChip>
          <span className="inline-flex items-center py-1 px-3 rounded bg-ink text-paper text-sm cursor-pointer" >Invite</span>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="flex items-start gap-3 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 mb-4" style={{
 borderLeft: "3px solid var(--accent)" }}>
          <span className="kanji text-base text-accent" style={{ lineHeight: 1.2 }}>任</span>
          <div className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>
            <b className="text-ink font-semibold" >Just-in-time on first connect.</b> A new member is provisioned automatically at their git-derived role — but the auto-default is capped at <b className="font-semibold" >Read-only</b>. Maintainer and Org admin are never granted automatically; they're elevated by hand below.
          </div>
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {!mobile && (
          <div className="grid gap-3 py-3 px-4 border-b text-xs uppercase text-ink-mute font-semibold" style={{ gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", letterSpacing: ".1em" }}>
            <span>Member</span><span>Git role</span><span>Dōjō role</span><span>Scopes</span><span>Active</span>
          </div>
          )}
          {D.members.map((mm, i) => mobile ? (
            <div className="py-3 px-4" key={mm.name} style={{ borderBottom: i < D.members.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <div className="flex items-center gap-2" >
                <Avatar name={mm.name} size={28} />
                <span className="text-sm text-ink flex-1 min-w-0" >{mm.name}</span>
                <button className="inline-flex items-center gap-1 bg-paper border border-paper-edge rounded py-1 px-2 cursor-pointer shrink-0" >
                  <span className="text-sm font-medium" style={{ color: roleTone[mm.dojo] || "var(--ink)" }}>{mm.dojo}</span>
                  <span className="text-xs text-ink-faint" >▾</span>
                </button>
              </div>
              <div className="mono text-xs text-ink-mute mt-1" style={{ paddingLeft: 36 }}>{mm.git} · {mm.scopes} · {mm.active}</div>
            </div>
          ) : (
            <div className="grid gap-3 py-3 px-4 items-center" key={mm.name} style={{ gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", borderBottom: i < D.members.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <div className="flex items-center gap-2" >
                <Avatar name={mm.name} size={28} />
                <span className="text-sm text-ink" >{mm.name}</span>
              </div>
              <span className="mono text-xs text-ink-mute" >{mm.git}</span>
              <button className="inline-flex items-center gap-1 bg-paper border border-paper-edge rounded py-1 px-2 cursor-pointer" style={{ justifySelf: "start" }}>
                <span className="text-sm font-medium" style={{ color: roleTone[mm.dojo] || "var(--ink)" }}>{mm.dojo}</span>
                <span className="text-xs text-ink-faint" >▾</span>
              </button>
              <span className="text-xs text-ink-soft" >{mm.scopes}</span>
              <span className="mono text-xs text-ink-faint" >{mm.active}</span>
            </div>
          ))}
        </div>
        <div className="flex items-start gap-2 mt-3 text-xs text-ink-mute" style={{ lineHeight: 1.5, maxWidth: 760 }}>
          <span className="kanji text-sm text-accent" >規</span>
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="規" eyebrow="Org · policy" title="Scopes &amp; policies"
        sub="The scope hierarchy and its attribution / confidentiality rules. Templates seed a scope; the precedence ladder resolves conflicts top-down. Client-origin lessons are anonymized universally — client, repo and source dropped — before any scope rule applies."
        right={<div className="flex gap-2" >
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Apply template ▾</DojoChip>
        </div>} />
      <div className="flex-1 overflow-auto grid items-start" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(340px, 1fr))", gap: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="flex flex-col gap-6 min-w-0" >
          <Panel title="Scope hierarchy" note="company → team → project → repo · stack">
            <div className="flex flex-col" >
              {tree.map((s, i) => (
                <div className="flex items-center gap-2 py-2 px-1" key={s.name} style={{
 paddingLeft: `calc(var(--space-1) + ${s.lvl} * var(--space-6))`, borderBottom: i < tree.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <span className="kanji text-sm text-center" style={{ color: s.client ? "var(--accent)" : "var(--ink-mute)", width: 18 }}>{s.k}</span>
                  <span className="text-sm text-ink flex-1" >{s.name}</span>
                  {s.client && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">anonymized</DojoChip>}
                  <DojoChip>{s.preset}</DojoChip>
                  <span className="mono text-xs text-ink-faint text-right" style={{ width: 54 }}>{s.share}</span>
                </div>
              ))}
            </div>
          </Panel>
          <Panel title="Test a candidate · which rule wins" note="simulate before you ship a policy">
            <div className="flex flex-col gap-3" >
              <div className="flex gap-2 flex-wrap" >
                <DojoChip tone="var(--ink-soft)">type · guard ▾</DojoChip>
                <DojoChip tone="var(--ink-soft)">bound · Repo ledger-core ▾</DojoChip>
                <DojoChip tone="var(--accent)" soft="var(--accent-soft)">origin · Client Globex ▾</DojoChip>
              </div>
              <div className="flex items-center gap-3 bg-paper rounded-lg py-3 px-3" style={{ border: "1px solid var(--accent-edge)" }}>
                <span className="kanji text-base text-accent" >守</span>
                <div className="text-sm text-ink" style={{ lineHeight: 1.5 }}>
                  <b className="font-semibold" >Client anonymization applies first</b> — client, repo and source are dropped. Then the <b className="font-semibold text-accent" >{routed ? routed.name : "Company"}</b> rule routes it{routed && routed.note ? ` (${routed.note})` : ""}.
                </div>
              </div>
            </div>
          </Panel>
        </div>
        <Panel title="Precedence ladder" note="reorder · top wins">
          <div className="flex flex-col gap-2" >
            {rungs.map((r, i) => {
              const isRouted = routed && r.name === routed.name;
              return (
              <div className="grid gap-2 items-center py-2 px-3 rounded-lg" key={r.name} style={{ gridTemplateColumns: "auto auto 1fr auto auto",
 background: r.locked ? "var(--accent-soft)" : isRouted ? "var(--paper-soft)" : "var(--paper)",
 border: r.locked ? "1px solid var(--accent-edge)" : isRouted ? "1px solid var(--accent-edge)" : "var(--hairline)" }}>
                <span className="mono text-xs text-ink-faint" style={{ width: 12 }}>{i + 1}</span>
                <div className="flex flex-col" style={{ gap: 1 }}>
                  <button className="border-0 text-xs p-0" onClick={() => move(i, -1)} disabled={r.locked || i <= 1} title="Move up" style={{ lineHeight: 0.7, background: "none", cursor: r.locked || i <= 1 ? "default" : "pointer", color: r.locked || i <= 1 ? "var(--ink-faint)" : "var(--ink-mute)", opacity: r.locked ? 0.3 : 1 }}>▲</button>
                  <button className="border-0 text-xs p-0" onClick={() => move(i, 1)} disabled={r.locked || i >= rungs.length - 1} title="Move down" style={{ lineHeight: 0.7, background: "none", cursor: r.locked || i >= rungs.length - 1 ? "default" : "pointer", color: r.locked || i >= rungs.length - 1 ? "var(--ink-faint)" : "var(--ink-mute)", opacity: r.locked ? 0.3 : 1 }}>▼</button>
                </div>
                <div className="min-w-0 flex items-baseline gap-2 flex-wrap" >
                  <span className="text-sm text-ink" >{r.name}</span>
                  {r.note && <span className="text-xs text-ink-faint" >{r.note}</span>}
                </div>
                {isRouted && <span className="kanji text-sm text-accent" >守</span>}
                {r.locked
                  ? <span className="mono text-xs text-accent uppercase" style={{ letterSpacing: ".08em" }}>pinned</span>
                  : isRouted
                  ? <span className="mono text-xs text-accent uppercase" style={{ letterSpacing: ".08em" }}>routes</span>
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
function DojoAdminConsole({ initial = "overview", mobile = false, relayStart = null, onExit, enteredOrg }) {
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
      nav={ADMIN_NAV} active={active} setActive={setActive} mobile={mobile} relayStart={relayStart} zone="dojo" onExit={onExit} orgOverride={enteredOrg}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoAdminConsole, DojoOverview, DojoMonitor, DojoMembers, DojoScopes });
