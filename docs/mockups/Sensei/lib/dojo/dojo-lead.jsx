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

/* ─── Clients · dereferencing oversight ──────────────────── */
function DojoClients({ go }) {
  const engagements = [
    { kanji: "客", name: "Globex", lessons: 86, scopes: "lumen-auth · billing" },
    { kanji: "客", name: "Initech", lessons: 56, scopes: "initech-portal" },
  ];
  const kept = [
    { k: "標", t: "Standards, patterns & anti-patterns", d: "the reusable rule itself" },
    { k: "憶", t: "The memory — what · why · impact", d: "the reasoning that makes it teachable" },
    { k: "例", t: "Examples, anonymized", d: "illustrative code with every identifier stripped" },
  ];
  const dropped = [
    { k: "客", t: "Client & engagement name" },
    { k: "庫", t: "Repo, file paths & URLs" },
    { k: "名", t: "Identifiers & named entities" },
    { k: "源", t: "The original source & links" },
  ];
  const Panel = ({ title, note, right, children }) => (
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
        <span style={{ flex: 1 }} />
        {right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="守" eyebrow="Trust · clients" title="Client confidentiality"
        sub="One model for every client and every org: keep the lesson, drop the source. Standards, patterns, anti-patterns and the memory behind them travel — repo, client and identifiers never do."
        right={<div style={{ textAlign: "right", fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--ink-3)", lineHeight: 1.7 }}>
          <div><b style={{ color: "var(--success)" }}>142</b> anonymized · 7d</div>
          <div style={{ color: "var(--success)" }}>0 incidents · uniform policy</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "flex", flexDirection: "column", gap: 20 }}>
        <Panel title="One universal model" note="applied identically to every engagement"
          right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">no per-client settings</DojoChip>}>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 0 }}>
            <div style={{ paddingRight: 22, borderRight: "1px solid var(--edge)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 13 }}>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />
                <span style={{ fontSize: 11, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--success)", fontWeight: 700 }}>Kept — travels upstream</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                {kept.map(x => (
                  <div key={x.t} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: 11, alignItems: "start" }}>
                    <span className="kanji" style={{ fontSize: 16, color: "var(--success)", width: 18, textAlign: "center" }}>{x.k}</span>
                    <div><div style={{ fontSize: 13.5, color: "var(--ink)" }}>{x.t}</div><div style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 2 }}>{x.d}</div></div>
                  </div>
                ))}
              </div>
            </div>
            <div style={{ paddingLeft: 22 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 13 }}>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--ink-4)" }} />
                <span style={{ fontSize: 11, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 700 }}>Dropped — never leaves</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                {dropped.map(x => (
                  <div key={x.t} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 11, alignItems: "center" }}>
                    <span className="kanji" style={{ fontSize: 16, color: "var(--ink-4)", width: 18, textAlign: "center" }}>{x.k}</span>
                    <span style={{ fontSize: 13.5, color: "var(--ink-2)", textDecoration: "line-through", textDecorationColor: "var(--ink-4)" }}>{x.t}</span>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>stripped</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Panel>
        <div style={{ display: "grid", gridTemplateColumns: "1.25fr 1fr", gap: 18, alignItems: "start" }}>
          <Panel title="Examples are kept — anonymized" note="raw → what actually leaves">
            <div style={{ border: "var(--hairline)", borderRadius: 8, overflow: "hidden" }}>
              <div style={{ display: "flex", gap: 9, padding: "9px 13px", borderBottom: "1px solid var(--edge)", fontFamily: "var(--font-mono)", fontSize: 11.5, background: "var(--paper)" }}>
                <span style={{ width: 10, flexShrink: 0, color: "var(--ink-4)", fontWeight: 700 }}>−</span>
                <span style={{ color: "var(--ink-3)", textDecoration: "line-through", textDecorationColor: "var(--ink-4)" }}>globex/lumen-auth · POST /v2/webhooks/billing · ACME_WEBHOOK_SECRET</span>
              </div>
              <div style={{ display: "flex", gap: 9, padding: "9px 13px", fontFamily: "var(--font-mono)", fontSize: 11.5, background: "var(--paper)" }}>
                <span style={{ width: 10, flexShrink: 0, color: "var(--success)", fontWeight: 700 }}>+</span>
                <span style={{ color: "var(--ink)" }}>verify the HMAC signature header against the shared secret, then parse the body</span>
              </div>
            </div>
            <div style={{ fontSize: 11.5, color: "var(--ink-3)", lineHeight: 1.5, marginTop: 11 }}>The teaching keeps a concrete example — the client, repo, route and secret name are dropped before it ever reaches a maintainer.</div>
          </Panel>
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
            <div style={{ display: "flex", alignItems: "flex-start", gap: 11, background: "var(--paper-2)", border: "var(--hairline)", borderLeft: "3px solid var(--accent)", borderRadius: 10, padding: "13px 15px" }}>
              <span className="kanji" style={{ fontSize: 16, color: "var(--accent)", lineHeight: 1.2 }}>盾</span>
              <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
                <b style={{ color: "var(--ink)", fontWeight: 600 }}>Can't be anonymized? It doesn't leave.</b> If a lesson can't stand without identifying context, it's dropped automatically — never weakened, never queued for a judgment call.
              </div>
            </div>
            <Panel title="Engagements" note="routing only"
              right={<button style={{ display: "inline-flex", alignItems: "center", gap: 6, padding: "5px 11px", borderRadius: 7, background: "var(--ink)", color: "var(--paper)", fontSize: 11.5, cursor: "pointer", fontFamily: "inherit", border: "none" }}>+ Register</button>}>
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {engagements.map((e, i) => (
                  <div key={e.name} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 11, alignItems: "center", padding: "9px 2px", borderBottom: i < engagements.length - 1 ? "1px solid var(--edge)" : "none" }}>
                    <span className="kanji" style={{ fontSize: 16, color: "var(--accent)", width: 18, textAlign: "center" }}>{e.kanji}</span>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontSize: 13, color: "var(--ink)" }}>{e.name}</div>
                      <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 2 }}>{e.scopes}</div>
                    </div>
                    <DojoChip tone="var(--success)" soft="var(--success-soft)">{e.lessons} anonymized</DojoChip>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 9, padding: "13px 16px", background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12 }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>録</span>
          <span style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.5, flex: 1 }}>Every anonymization is written to the immutable audit trail — the proof the source was dropped, per engagement, exportable as a confidentiality report.</span>
          <button onClick={() => go && go("audit")} className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", whiteSpace: "nowrap" }}>open audit trail →</button>
        </div>
      </div>
    </div>
  );
}

/* ─── Audit trail · confidentiality ledger ───────────────── */
function DojoAudit() {
  const log = [
    { t: "09:42", ev: "Dereference", tone: "accent", lesson: "Validate webhook signature before parsing", client: "Globex", actor: "system", hash: "a3f9c1" },
    { t: "09:40", ev: "Outbound", tone: "ink", lesson: "Idempotency key on money-moving mutations", client: "Globex", actor: "Keiko T.", hash: "b1c7e0" },
    { t: "08:55", ev: "Exception cleared", tone: "success", lesson: "Retry budget for a billing webhook", client: "Globex", actor: "Mei L.", hash: "77d24b" },
    { t: "Yest · 18:03", ev: "Quarantine", tone: "warn", lesson: "Cache key shape for a multi-tenant lookup", client: "Initech", actor: "leak-guard", hash: "0e4a8f" },
    { t: "Yest · 11:20", ev: "Dereference", tone: "accent", lesson: "Exponential backoff schedule", client: "Initech", actor: "system", hash: "5fb831" },
    { t: "Mon · 16:47", ev: "Outbound", tone: "ink", lesson: "Persona: integration-test author for auth", client: "—", actor: "Sven K.", hash: "c920a6" },
  ];
  const evTone = { accent: "var(--accent)", ink: "var(--ink-2)", success: "var(--success)", warn: "oklch(0.52 0.13 60)" };
  const evSoft = { accent: "var(--accent-soft)", ink: "var(--paper-3)", success: "var(--success-soft)", warn: "var(--warning-soft)" };
  const steps = [
    { k: "警", name: "Alert", note: "leak-guard fires" },
    { k: "隔", name: "Quarantine", note: "lesson held" },
    { k: "退", name: "Retract", note: "pull downstream" },
    { k: "省", name: "Review", note: "post-incident" },
  ];
  const access = [{ name: "Globex", on: true }, { name: "Initech", on: false }];
  const Panel = ({ title, note, right, children }) => (
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
        <span style={{ flex: 1 }} />
        {right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="録" eyebrow="Trust · audit" title="Confidentiality audit trail"
        sub="An immutable record of every dereference, outbound lesson, and decision — per client. The proof that confidentiality held."
        right={<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Filter · all clients ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6, padding: "6px 12px", borderRadius: 7, background: "var(--ink)", color: "var(--paper)", fontSize: 12.5, cursor: "pointer" }}>Export report</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "flex", flexDirection: "column", gap: 20 }}>
        <Panel title="Incident response" note="alert → quarantine → retract → review"
          right={<><DojoChip tone="var(--ink-3)">shared with admin · Monitor</DojoChip><DojoChip tone="var(--success)" soft="var(--success-soft)">no active incidents · armed</DojoChip></>}>
          <div style={{ display: "flex", alignItems: "stretch", gap: 0 }}>
            {steps.map((s, i) => (
              <React.Fragment key={s.name}>
                <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 5, padding: "4px 8px" }}>
                  <span className="kanji" style={{ fontSize: 22, color: "var(--accent)", opacity: 0.85 }}>{s.k}</span>
                  <span style={{ fontSize: 13, color: "var(--ink)", fontWeight: 500 }}>{s.name}</span>
                  <span style={{ fontSize: 11, color: "var(--ink-4)" }}>{s.note}</span>
                </div>
                {i < steps.length - 1 && <span style={{ alignSelf: "center", fontSize: 15, color: "var(--ink-4)" }}>→</span>}
              </React.Fragment>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 12, paddingTop: 12, borderTop: "1px solid var(--edge)", fontSize: 12, color: "var(--ink-3)", lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>警</span>
            <span>The client lead and org admin are notified immediately; a severity tier decides whether the client is told, per the engagement's contract.</span>
          </div>
        </Panel>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
          <Panel title="Retention" note="per engagement">
            <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>Follows each engagement's contract, plus any statutory minimum.</div>
            <div style={{ display: "flex", gap: 18, marginTop: 11 }}>
              {[["Globex", "term + 1y"], ["Initech", "term"]].map(([n, v]) => (
                <div key={n}><div className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>客 {n}</div><div style={{ fontSize: 13.5, color: "var(--ink)", marginTop: 2 }}>{v}</div></div>
              ))}
            </div>
          </Panel>
          <Panel title="Client read access" note="their own log only">
            <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55, marginBottom: 11 }}>A client can be granted read-only access to its own confidentiality log.</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {access.map(a => (
                <div key={a.name} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}><span className="kanji" style={{ color: "var(--accent)" }}>客</span> {a.name}</span>
                  <span style={{ display: "inline-flex", alignItems: "center", width: 38, height: 20, borderRadius: 12, padding: 2,
                              background: a.on ? "var(--accent)" : "var(--paper-3)", justifyContent: a.on ? "flex-end" : "flex-start" }}>
                    <span style={{ width: 16, height: 16, borderRadius: "50%", background: "var(--paper)" }} />
                  </span>
                  <span className="mono" style={{ fontSize: 10, width: 26, color: a.on ? "var(--accent)" : "var(--ink-4)", textTransform: "uppercase" }}>{a.on ? "on" : "off"}</span>
                </div>
              ))}
            </div>
          </Panel>
        </div>
        <Panel title="Ledger" note="immutable · hash-chained" right={<span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>144 entries · 7d</span>}>
          <div style={{ display: "grid", gridTemplateColumns: "92px 130px 1fr 96px 96px 78px", gap: 12, padding: "0 4px 9px",
                        fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600 }}>
            <span>Time</span><span>Event</span><span>Lesson</span><span>Client</span><span>Actor</span><span>Hash</span>
          </div>
          <div style={{ display: "flex", flexDirection: "column" }}>
            {log.map((e, i) => (
              <div key={i} style={{ display: "grid", gridTemplateColumns: "92px 130px 1fr 96px 96px 78px", gap: 12, alignItems: "center",
                            padding: "10px 4px", borderTop: "1px solid var(--edge)" }}>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{e.t}</span>
                <span><DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip></span>
                <span style={{ fontSize: 12.5, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{e.lesson}</span>
                <span style={{ fontSize: 12, color: "var(--ink-2)" }}>{e.client}</span>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{e.actor}</span>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", display: "inline-flex", alignItems: "center", gap: 4 }}>⠿{e.hash}</span>
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── the lead console ───────────────────────────────────── */
function DojoLeadConsole({ initial = "clients" }) {
  const [active, setActive] = dlS(initial);
  const go = (s) => { if (LEAD_SECTIONS.includes(s)) setActive(s); };
  const screen = active === "audit" ? <DojoAudit /> : <DojoClients go={go} />;
  return (
    <DojoRoleShell label="Dōjō · Lead console" role={{ kanji: "客", label: "Client lead" }}
      nav={LEAD_NAV} active={active} setActive={setActive}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoLeadConsole, DojoClients, DojoAudit });
