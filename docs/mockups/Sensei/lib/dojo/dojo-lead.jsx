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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="守" eyebrow="Trust · clients" title="Client confidentiality"
        sub="One model for every client and every org: keep the lesson, drop the source. Standards, patterns, anti-patterns and the memory behind them travel — repo, client and identifiers never do."
        right={<div className="text-right text-xs text-ink-mute" style={{ fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div><b className="text-success" >142</b> anonymized · 7d</div>
          <div className="text-success" >0 incidents · uniform policy</div>
        </div>} />
      <div className="flex-1 overflow-auto flex flex-col gap-4" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <Panel title="One universal model" note="applied identically to every engagement"
          right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">no per-client settings</DojoChip>}>
          <div className="grid" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 1fr", gap: mobile ? "var(--space-4)" : 0 }}>
            <div style={mobile ? { paddingBottom: "var(--space-4)", borderBottom: "1px solid var(--paper-edge)" } : { paddingRight: "var(--space-6)", borderRight: "1px solid var(--paper-edge)" }}>
              <div className="flex items-center gap-2 mb-3" >
                <span className="rounded-full bg-success" style={{ width: 7, height: 7 }} />
                <span className="text-xs uppercase text-success" style={{ letterSpacing: ".1em", fontWeight: 700 }}>Kept — travels upstream</span>
              </div>
              <div className="flex flex-col gap-3" >
                {kept.map(x => (
                  <div className="grid gap-3 items-start" key={x.t} style={{ gridTemplateColumns: "auto 1fr" }}>
                    <span className="kanji text-base text-success text-center" style={{ width: 18 }}>{x.k}</span>
                    <div><div className="text-sm text-ink" >{x.t}</div><div className="text-xs text-ink-mute mt-1" >{x.d}</div></div>
                  </div>
                ))}
              </div>
            </div>
            <div style={mobile ? {} : { paddingLeft: "var(--space-6)" }}>
              <div className="flex items-center gap-2 mb-3" >
                <span className="rounded-full bg-ink-faint" style={{ width: 7, height: 7 }} />
                <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".1em", fontWeight: 700 }}>Dropped — never leaves</span>
              </div>
              <div className="flex flex-col gap-3" >
                {dropped.map(x => (
                  <div className="grid gap-3 items-center" key={x.t} style={{ gridTemplateColumns: "auto 1fr auto" }}>
                    <span className="kanji text-base text-ink-faint text-center" style={{ width: 18 }}>{x.k}</span>
                    <span className="text-sm text-ink-soft" style={{ textDecoration: "line-through", textDecorationColor: "var(--ink-faint)" }}>{x.t}</span>
                    <span className="mono text-xs text-ink-faint" >dropped</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Panel>
        <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(340px, 1fr))" }}>
          <Panel title="Examples are kept — anonymized" note="raw → what actually leaves">
            <div className="border border-paper-edge rounded-lg overflow-hidden" >
              <div className="flex gap-2 py-2 px-3 text-xs bg-paper" style={{ borderBottom: "1px solid var(--paper-edge)", fontFamily: "var(--font-mono)" }}>
                <span className="shrink-0 text-ink-faint" style={{ width: 10, fontWeight: 700 }}>−</span>
                <span className="text-ink-mute" style={{ textDecoration: "line-through", textDecorationColor: "var(--ink-faint)" }}>globex/lumen-auth · POST /v2/webhooks/billing · ACME_WEBHOOK_SECRET</span>
              </div>
              <div className="flex gap-2 py-2 px-3 text-xs bg-paper" style={{ fontFamily: "var(--font-mono)" }}>
                <span className="shrink-0 text-success" style={{ width: 10, fontWeight: 700 }}>+</span>
                <span className="text-ink" >verify the HMAC signature header against the shared secret, then parse the body</span>
              </div>
            </div>
            <div className="text-xs text-ink-mute mt-3" style={{ lineHeight: 1.5 }}>The teaching keeps a concrete example — the client, repo, route and secret name are dropped before it ever reaches a maintainer.</div>
          </Panel>
          <div className="flex flex-col gap-3" >
            <div className="flex items-start gap-3 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" style={{ borderLeft: "3px solid var(--accent)" }}>
              <span className="kanji text-base text-accent" style={{ lineHeight: 1.2 }}>盾</span>
              <div className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>
                <b className="text-ink font-semibold" >Can't be anonymized? It doesn't leave.</b> If a lesson can't stand without identifying context, it's dropped automatically — never weakened, never queued for a judgment call.
              </div>
            </div>
            <Panel title="Engagements" note="routing only"
              right={<button className="inline-flex items-center gap-1 py-1 px-3 rounded bg-ink text-paper text-xs cursor-pointer border-0" style={{ fontFamily: "inherit" }}>+ Register</button>}>
              <div className="flex flex-col gap-1" >
                {engagements.map((e, i) => (
                  <div className="grid gap-3 items-center py-2 px-1" key={e.name} style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: i < engagements.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                    <span className="kanji text-base text-accent text-center" style={{ width: 18 }}>{e.kanji}</span>
                    <div className="min-w-0" >
                      <div className="text-sm text-ink" >{e.name}</div>
                      <div className="mono text-xs text-ink-faint mt-1" >{e.scopes}</div>
                    </div>
                    <DojoChip tone="var(--success)" soft="var(--success-soft)">{e.lessons} anonymized</DojoChip>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        </div>
        <div className="flex items-center gap-2 py-3 px-4 bg-paper-soft border border-paper-edge rounded-lg" >
          <span className="kanji text-base text-accent" >録</span>
          <span className="text-sm text-ink-soft flex-1" style={{ lineHeight: 1.5 }}>Every anonymization is written to the immutable audit trail — the proof the source was dropped, per engagement, exportable as a confidentiality report.</span>
          <button onClick={() => go && go("audit")} className="mono text-xs text-accent border-0 cursor-pointer whitespace-nowrap" style={{ background: "none" }}>open audit trail →</button>
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="録" eyebrow="Trust · audit" title="Confidentiality audit trail"
        sub="An immutable record of every anonymize, outbound lesson, and decision — per client. The proof that confidentiality held."
        right={<div className="flex gap-2 items-center" >
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Filter · all clients ▾</DojoChip>
          <span className="inline-flex items-center gap-1 py-1 px-3 rounded bg-ink text-paper text-sm cursor-pointer" >Export report</span>
        </div>} />
      <div className="flex-1 overflow-auto flex flex-col gap-4" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <Panel title="Incident response" note="alert → quarantine → retract → review"
          right={<><DojoChip tone="var(--ink-mute)">shared with admin · Monitor</DojoChip><DojoChip tone="var(--success)" soft="var(--success-soft)">no active incidents · armed</DojoChip></>}>
          <div className="flex items-stretch" style={{ flexDirection: mobile ? "column" : "row", gap: mobile ? "var(--space-2)" : 0 }}>
            {steps.map((s, i) => (
              <React.Fragment key={s.name}>
                <div className="flex-1 flex items-center py-1 px-2" style={{ flexDirection: mobile ? "row" : "column", gap: mobile ? "var(--space-3)" : "var(--space-1)" }}>
                  <span className="kanji text-xl text-accent" style={{ opacity: 0.85 }}>{s.k}</span>
                  <span className="text-sm text-ink font-medium" >{s.name}</span>
                  <span className="text-xs text-ink-faint" >{s.note}</span>
                </div>
                {!mobile && i < steps.length - 1 && <span className="self-center text-base text-ink-faint" >→</span>}
              </React.Fragment>
            ))}
          </div>
          <div className="flex items-center gap-2 mt-3 pt-3 text-xs text-ink-mute" style={{ borderTop: "1px solid var(--paper-edge)", lineHeight: 1.5 }}>
            <span className="kanji text-sm text-accent" >警</span>
            <span>The client lead and org admin are notified immediately; a severity tier decides whether the client is told, per the engagement's contract.</span>
          </div>
        </Panel>
        <div className="grid gap-3" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))" }}>
          <Panel title="Retention" note="per engagement">
            <div className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>Follows each engagement's contract, plus any statutory minimum.</div>
            <div className="flex gap-4 mt-3" >
              {[["Globex", "term + 1y"], ["Initech", "term"]].map(([n, v]) => (
                <div key={n}><div className="mono text-xs text-ink-faint" >客 {n}</div><div className="text-sm text-ink mt-1" >{v}</div></div>
              ))}
            </div>
          </Panel>
          <Panel title="Client read access" note="their own log only">
            <div className="text-sm text-ink-soft mb-3" style={{ lineHeight: 1.55 }}>A client can be granted read-only access to its own confidentiality log.</div>
            <div className="flex flex-col gap-2" >
              {access.map(a => (
                <div className="flex items-center gap-2" key={a.name} >
                  <span className="text-sm text-ink flex-1" ><span className="kanji text-accent" >客</span> {a.name}</span>
                  <span className="inline-flex items-center rounded-lg p-1" style={{ width: 38, height: 20,
 background: a.on ? "var(--accent)" : "var(--paper-mute)", justifyContent: a.on ? "flex-end" : "flex-start" }}>
                    <span className="rounded-full bg-paper" style={{ width: 16, height: 16 }} />
                  </span>
                  <span className="mono text-xs uppercase" style={{ width: 26, color: a.on ? "var(--accent)" : "var(--ink-faint)" }}>{a.on ? "on" : "off"}</span>
                </div>
              ))}
            </div>
          </Panel>
        </div>
        <Panel title="Ledger" note="immutable · hash-chained" right={<span className="mono text-xs text-ink-faint" >144 entries · 7d</span>}>
          <div className="flex gap-2 flex-wrap mb-3" >
            {["all", ...Array.from(new Set(allLog.map(e => e.ev)))].map(ev => {
              const on = evFilter === ev;
              return (
                <button className="cursor-pointer rounded-full py-1 px-3 text-xs" key={ev} onClick={() => setEvFilter(ev)} style={{ fontFamily: "inherit",
 border: on ? "1px solid var(--ink)" : "var(--hairline)",
 background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)" }}>{ev === "all" ? "All events" : ev}</button>
              );
            })}
          </div>
          {!mobile && (
          <div className="grid gap-3 text-xs uppercase text-ink-faint font-semibold" style={{ gridTemplateColumns: "92px 130px minmax(220px,1fr) 96px 96px 78px", padding: "0 var(--space-1) var(--space-2)", letterSpacing: ".1em" }}>
            <span>Time</span><span>Event</span><span>Lesson</span><span>Client</span><span>Actor</span><span>Hash</span>
          </div>
          )}
          <div className="flex flex-col" >
            {log.length === 0
              ? <div className="py-4 px-1 text-sm text-ink-faint italic" >No {evFilter} events in this window.</div>
              : log.map((e, i) => mobile ? (
              <div className="py-3 px-1" key={i} style={{ borderTop: "1px solid var(--paper-edge)" }}>
                <div className="flex items-center gap-2 flex-wrap" >
                  <DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip>
                  <span className="mono text-xs text-ink-faint" >{e.t}</span>
                  <span className="flex-1" />
                  <span className="mono text-xs text-ink-faint" >⠿{e.hash}</span>
                </div>
                <div className="text-sm text-ink mt-1" >{e.lesson}</div>
                <div className="mono text-xs text-ink-mute mt-1" >{e.client} · {e.actor}</div>
              </div>
            ) : (
              <div className="grid gap-3 items-center py-2 px-1" key={i} style={{ gridTemplateColumns: "92px 130px minmax(220px,1fr) 96px 96px 78px", borderTop: "1px solid var(--paper-edge)" }}>
                <span className="mono text-xs text-ink-faint" >{e.t}</span>
                <span><DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip></span>
                <span className="text-sm text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{e.lesson}</span>
                <span className="text-xs text-ink-soft" >{e.client}</span>
                <span className="mono text-xs text-ink-mute" >{e.actor}</span>
                <span className="mono text-xs text-ink-faint inline-flex items-center gap-1" >⠿{e.hash}</span>
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── the lead console ───────────────────────────────────── */
function DojoLeadConsole({ initial = "clients", mobile = false, relayStart = null, onExit, enteredOrg }) {
  const [active, setActive] = dlS(initial);
  const go = (s) => { if (LEAD_SECTIONS.includes(s)) setActive(s); };
  const screen = active === "audit" ? <DojoAudit mobile={mobile} /> : <DojoClients go={go} mobile={mobile} />;
  return (
    <DojoRoleShell label="Dōjō · Lead console" role={{ kanji: "客", label: "Client lead" }}
      nav={LEAD_NAV} active={active} setActive={setActive} mobile={mobile} relayStart={relayStart} zone="dojo" onExit={onExit} orgOverride={enteredOrg}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoLeadConsole, DojoClients, DojoAudit });
