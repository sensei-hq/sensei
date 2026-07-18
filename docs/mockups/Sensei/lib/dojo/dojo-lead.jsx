// Dōjō · Lead console — client engagements & the confidentiality record.
// Panels: Clients · Audit. Job: define the engagement, anonymize always (no
// per-item review), watch the immutable audit trail, and handle incidents.
// Reuses the shared frame from dojo-shared.jsx.

const { useState: dlS } = React;

const LEAD_NAV = [
  { group: "Trust", items: [
    { id: "clients", kanji: "守", label: "Clients" },
    { id: "audit",   kanji: "録", label: "Audit trail" },
  ]},
];
const LEAD_SECTIONS = ["clients", "audit"];

/* ─── Clients · anonymization oversight ──────────────────── */
function DojoClients({ go, mobile = false }) {
  const engagements = [
    { kanji: "客", name: "Globex", lessons: 86, scopes: "lumen-auth · billing" },
    { kanji: "客", name: "Initech", lessons: 56, scopes: "initech-portal" },
  ];
  const kept = [
    { k: "標", t: "Standards, patterns & anti-patterns", d: "the reusable rule itself" },
    { k: "憶", t: "The memory — what · why · impact", d: "the reasoning that makes it teachable" },
    { k: "例", t: "Examples, anonymized", d: "illustrative code with every identifier dropped" },
  ];
  const dropped = [
    { k: "客", t: "Client & engagement name" },
    { k: "庫", t: "Repo, file paths & URLs" },
    { k: "名", t: "Identifiers & named entities" },
    { k: "源", t: "The original source & links" },
  ];
  const Panel = (props) => <window.DojoPanel {...props} />;
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="守" eyebrow="Trust · clients" title="Client confidentiality"
        sub="One model for every client and every org: keep the lesson, drop the source. Standards, patterns, anti-patterns and the memory behind them travel — repo, client and identifiers never do."
        right={<div style={{ textAlign: "right", fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)", color: "var(--ink-mute)", lineHeight: 1.7 }}>
          <div><b style={{ color: "var(--success)" }}>142</b> anonymized · 7d</div>
          <div style={{ color: "var(--success)" }}>0 incidents · uniform policy</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)", display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
        <Panel title="One universal model" note="applied identically to every engagement"
          right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">no per-client settings</DojoChip>}>
          <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr", gap: mobile ? "var(--space-4)" : 0 }}>
            <div style={mobile ? { paddingBottom: "var(--space-4)", borderBottom: "1px solid var(--paper-edge)" } : { paddingRight: "var(--space-5)", borderRight: "1px solid var(--paper-edge)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--success)", fontWeight: 700 }}>Kept — travels upstream</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                {kept.map(x => (
                  <div key={x.t} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "var(--space-3)", alignItems: "start" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--success)", width: 18, textAlign: "center" }}>{x.k}</span>
                    <div><div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{x.t}</div><div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{x.d}</div></div>
                  </div>
                ))}
              </div>
            </div>
            <div style={mobile ? {} : { paddingLeft: "var(--space-5)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--ink-faint)" }} />
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 700 }}>Dropped — never leaves</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                {dropped.map(x => (
                  <div key={x.t} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--ink-faint)", width: 18, textAlign: "center" }}>{x.k}</span>
                    <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", textDecoration: "line-through", textDecorationColor: "var(--ink-faint)" }}>{x.t}</span>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>dropped</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Panel>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(340px, 1fr))", gap: "var(--space-4)", alignItems: "start" }}>
          <Panel title="Examples are kept — anonymized" note="raw → what actually leaves">
            <div style={{ border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
              <div style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-2) var(--space-3)", borderBottom: "1px solid var(--paper-edge)", fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)", background: "var(--paper)" }}>
                <span style={{ width: 10, flexShrink: 0, color: "var(--ink-faint)", fontWeight: 700 }}>−</span>
                <span style={{ color: "var(--ink-mute)", textDecoration: "line-through", textDecorationColor: "var(--ink-faint)" }}>globex/lumen-auth · POST /v2/webhooks/billing · ACME_WEBHOOK_SECRET</span>
              </div>
              <div style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-2) var(--space-3)", fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)", background: "var(--paper)" }}>
                <span style={{ width: 10, flexShrink: 0, color: "var(--success)", fontWeight: 700 }}>+</span>
                <span style={{ color: "var(--ink)" }}>verify the HMAC signature header against the shared secret, then parse the body</span>
              </div>
            </div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, marginTop: "var(--space-3)" }}>The teaching keeps a concrete example — the client, repo, route and secret name are dropped before it ever reaches a maintainer.</div>
          </Panel>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
            <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-3)", background: "var(--paper-soft)", border: "var(--hairline)", borderLeft: "3px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", lineHeight: 1.2 }}>盾</span>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
                <b style={{ color: "var(--ink)", fontWeight: 600 }}>Can't be anonymized? It doesn't leave.</b> If a lesson can't stand without identifying context, it's dropped automatically — never weakened, never queued for a judgment call.
              </div>
            </div>
            <Panel title="Engagements" note="routing only"
              right={<button style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit", border: "none" }}>+ Register</button>}>
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
                {engagements.map((e, i) => (
                  <div key={e.name} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-2) var(--space-1)", borderBottom: i < engagements.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", width: 18, textAlign: "center" }}>{e.kanji}</span>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{e.name}</div>
                      <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{e.scopes}</div>
                    </div>
                    <DojoChip tone="var(--success)" soft="var(--success-soft)">{e.lessons} anonymized</DojoChip>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>録</span>
          <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.5, flex: 1 }}>Every anonymization is written to the immutable audit trail — the proof the source was dropped, per engagement, exportable as a confidentiality report.</span>
          <button onClick={() => go && go("audit")} className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer", whiteSpace: "nowrap" }}>open audit trail →</button>
        </div>
      </div>
    </div>
  );
}

/* ─── Audit trail · confidentiality ledger ───────────────── */
function DojoAudit({ mobile = false }) {
  const allLog = [
    { t: "09:42", ev: "Anonymize", tone: "accent", lesson: "Validate webhook signature before parsing", client: "Globex", actor: "system", hash: "a3f9c1" },
    { t: "09:40", ev: "Outbound", tone: "ink", lesson: "Idempotency key on money-moving mutations", client: "Globex", actor: "Keiko T.", hash: "b1c7e0" },
    { t: "08:55", ev: "Exception cleared", tone: "success", lesson: "Retry budget for a billing webhook", client: "Globex", actor: "Mei L.", hash: "77d24b" },
    { t: "Yest · 18:03", ev: "Quarantine", tone: "warn", lesson: "Cache key shape for a multi-tenant lookup", client: "Initech", actor: "leak-guard", hash: "0e4a8f" },
    { t: "Yest · 11:20", ev: "Anonymize", tone: "accent", lesson: "Exponential backoff schedule", client: "Initech", actor: "system", hash: "5fb831" },
    { t: "Mon · 16:47", ev: "Outbound", tone: "ink", lesson: "Persona: integration-test author for auth", client: "—", actor: "Sven K.", hash: "c920a6" },
  ];
  const [evFilter, setEvFilter] = dlS("all");
  const log = evFilter === "all" ? allLog : allLog.filter(e => e.ev === evFilter);
  const evTone = { accent: "var(--accent)", ink: "var(--ink-soft)", success: "var(--success)", warn: "var(--warning)" };
  const evSoft = { accent: "var(--accent-soft)", ink: "var(--paper-mute)", success: "var(--success-soft)", warn: "var(--warning-soft)" };
  const steps = [
    { k: "警", name: "Alert", note: "leak-guard fires" },
    { k: "隔", name: "Quarantine", note: "lesson held" },
    { k: "退", name: "Retract", note: "pull downstream" },
    { k: "省", name: "Review", note: "post-incident" },
  ];
  const access = [{ name: "Globex", on: true }, { name: "Initech", on: false }];
  const Panel = window.DojoPanel;
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="録" eyebrow="Trust · audit" title="Confidentiality audit trail"
        sub="An immutable record of every anonymize, outbound lesson, and decision — per client. The proof that confidentiality held."
        right={<div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Filter · all clients ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", cursor: "pointer" }}>Export report</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)", display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
        <Panel title="Incident response" note="alert → quarantine → retract → review"
          right={<><DojoChip tone="var(--ink-mute)">shared with admin · Monitor</DojoChip><DojoChip tone="var(--success)" soft="var(--success-soft)">no active incidents · armed</DojoChip></>}>
          <div style={{ display: "flex", flexDirection: mobile ? "column" : "row", alignItems: "stretch", gap: mobile ? "var(--space-2)" : 0 }}>
            {steps.map((s, i) => (
              <React.Fragment key={s.name}>
                <div style={{ flex: 1, display: "flex", flexDirection: mobile ? "row" : "column", alignItems: "center", gap: mobile ? "var(--space-3)" : "var(--space-1)", padding: "var(--space-1) var(--space-2)" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", opacity: 0.85 }}>{s.k}</span>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 500 }}>{s.name}</span>
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{s.note}</span>
                </div>
                {!mobile && i < steps.length - 1 && <span style={{ alignSelf: "center", fontSize: "var(--text-base)", color: "var(--ink-faint)" }}>→</span>}
              </React.Fragment>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-3)", paddingTop: "var(--space-3)", borderTop: "1px solid var(--paper-edge)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>警</span>
            <span>The client lead and org admin are notified immediately; a severity tier decides whether the client is told, per the engagement's contract.</span>
          </div>
        </Panel>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "var(--space-3)" }}>
          <Panel title="Retention" note="per engagement">
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>Follows each engagement's contract, plus any statutory minimum.</div>
            <div style={{ display: "flex", gap: "var(--space-4)", marginTop: "var(--space-3)" }}>
              {[["Globex", "term + 1y"], ["Initech", "term"]].map(([n, v]) => (
                <div key={n}><div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>客 {n}</div><div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", marginTop: "var(--space-1)" }}>{v}</div></div>
              ))}
            </div>
          </Panel>
          <Panel title="Client read access" note="their own log only">
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, marginBottom: "var(--space-3)" }}>A client can be granted read-only access to its own confidentiality log.</div>
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              {access.map(a => (
                <div key={a.name} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1 }}><span className="kanji" style={{ color: "var(--accent)" }}>客</span> {a.name}</span>
                  <span style={{ display: "inline-flex", alignItems: "center", width: 38, height: 20, borderRadius: "var(--radius-lg)", padding: "var(--space-1)",
                              background: a.on ? "var(--accent)" : "var(--paper-mute)", justifyContent: a.on ? "flex-end" : "flex-start" }}>
                    <span style={{ width: 16, height: 16, borderRadius: "50%", background: "var(--paper)" }} />
                  </span>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", width: 26, color: a.on ? "var(--accent)" : "var(--ink-faint)", textTransform: "uppercase" }}>{a.on ? "on" : "off"}</span>
                </div>
              ))}
            </div>
          </Panel>
        </div>
        <Panel title="Ledger" note="immutable · hash-chained" right={<span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>144 entries · 7d</span>}>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", marginBottom: "var(--space-3)" }}>
            {["all", ...Array.from(new Set(allLog.map(e => e.ev)))].map(ev => {
              const on = evFilter === ev;
              return (
                <button key={ev} onClick={() => setEvFilter(ev)} style={{ cursor: "pointer", fontFamily: "inherit",
                  border: on ? "1px solid var(--ink)" : "var(--hairline)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-3)", fontSize: "var(--text-xs)",
                  background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)" }}>{ev === "all" ? "All events" : ev}</button>
              );
            })}
          </div>
          {!mobile && (
          <div style={{ display: "grid", gridTemplateColumns: "92px 130px minmax(220px,1fr) 96px 96px 78px", gap: "var(--space-3)", padding: "0 var(--space-1) var(--space-2)",
                        fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600 }}>
            <span>Time</span><span>Event</span><span>Lesson</span><span>Client</span><span>Actor</span><span>Hash</span>
          </div>
          )}
          <div style={{ display: "flex", flexDirection: "column" }}>
            {log.length === 0
              ? <div style={{ padding: "var(--space-4) var(--space-1)", fontSize: "var(--text-sm)", color: "var(--ink-faint)", fontStyle: "italic" }}>No {evFilter} events in this window.</div>
              : log.map((e, i) => mobile ? (
              <div key={i} style={{ padding: "var(--space-3) var(--space-1)", borderTop: "1px solid var(--paper-edge)" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
                  <DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{e.t}</span>
                  <span style={{ flex: 1 }} />
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>⠿{e.hash}</span>
                </div>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", marginTop: "var(--space-1)" }}>{e.lesson}</div>
                <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{e.client} · {e.actor}</div>
              </div>
            ) : (
              <div key={i} style={{ display: "grid", gridTemplateColumns: "92px 130px minmax(220px,1fr) 96px 96px 78px", gap: "var(--space-3)", alignItems: "center",
                            padding: "var(--space-2) var(--space-1)", borderTop: "1px solid var(--paper-edge)" }}>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{e.t}</span>
                <span><DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip></span>
                <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{e.lesson}</span>
                <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{e.client}</span>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{e.actor}</span>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", display: "inline-flex", alignItems: "center", gap: "var(--space-1)" }}>⠿{e.hash}</span>
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── the lead console ───────────────────────────────────── */
function DojoLeadConsole({ initial = "clients", mobile = false, relayStart = null }) {
  const [active, setActive] = dlS(initial);
  const go = (s) => { if (LEAD_SECTIONS.includes(s)) setActive(s); };
  const screen = active === "audit" ? <DojoAudit mobile={mobile} /> : <DojoClients go={go} mobile={mobile} />;
  return (
    <DojoRoleShell label="Dōjō · Lead console" role={{ kanji: "客", label: "Client lead" }}
      nav={LEAD_NAV} active={active} setActive={setActive} mobile={mobile} relayStart={relayStart}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoLeadConsole, DojoClients, DojoAudit });
