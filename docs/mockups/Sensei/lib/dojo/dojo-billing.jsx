// Dōjō · Plan & billing (Org admin) — the business-model surface.
//
// The line: free where public or personal, paid where private & shared. Relay
// is FREE for individuals on their own projects; the shared-team parts of Relay
// (shared queue, presence, higher concurrency, approval audit) are what the
// paid tiers add. Seats are billed per ACTIVE CONTRIBUTOR — read-only is free.
// The app, the global Collective, and BYOK inference are always free.
//
// Reuses DojoHead / DojoChip from dojo-shared.jsx. Token-only → theme-free.

const { useState: blS } = React;

const BILL_TIERS = [
  { id: "free", kanji: "無", name: "Free", price: "Free", sub: "public · OSS · personal",
    lines: ["Public / open-source or personal solo Dōjō", "Unlimited members · full governance authoring",
            "Relay for your own projects — watch · approve · decide · chat", "Fair use: 1 active machine · standard realtime"] },
  { id: "team", kanji: "組", name: "Team", price: "Per seat", sub: "/ mo · active contributor", current: true,
    lines: ["Private, shared scopes for a company or team", "Role consoles · client engagements · audit",
            "Relay across the team — shared inbox, presence, priority realtime", "Read-only members always free"] },
  { id: "ent", kanji: "企", name: "Enterprise", price: "Contract", sub: "custom", dark: true,
    lines: ["Self-hosted / VPC · SSO (OIDC / SAML) + SCIM", "Audit retention & export · air-gapped bundle",
            "Self-hosted relay · SSO on mobile · approval-trail retention", "SLA & priority support"] },
];

function BillRelayRow({ label, free, tone }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-2) 0", borderBottom: "1px solid var(--paper-edge)" }}>
      <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>{label}</span>
      <DojoChip tone={tone} soft={free ? "var(--success-soft)" : "var(--accent-soft)"}>{free ? "free · individuals" : "paid · team"}</DojoChip>
    </div>
  );
}

function DojoBilling({ past = false, mobile = false }) {
  const seatsActive = 34, seatsReadonly = 14, perSeat = 12;
  const monthly = seatsActive * perSeat;
  const invoices = [
    { d: "Jul 1, 2026", amt: `$${monthly}.00`, s: "paid" },
    { d: "Jun 1, 2026", amt: "$396.00", s: "paid" },
    { d: "May 1, 2026", amt: "$372.00", s: "paid" },
  ];
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="円" eyebrow="Org · plan & billing" title="Plan & billing"
        sub="Free where public or personal; paid where private and shared. Seats are billed per active contributor — read-only members, the desktop app, the global Collective and bring-your-own-key inference are always free."
        right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">Team · private</DojoChip>} />

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
        {past && (
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", flexWrap: mobile ? "wrap" : "nowrap", background: "var(--danger-soft)", border: "1px solid var(--danger-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--danger)" }}>滞</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>Payment past due · $408 unpaid</div>
              <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", marginTop: "var(--space-1)", lineHeight: 1.5 }}>The Jul 1 charge failed. Private scopes stay active for <b style={{ fontWeight: 600 }}>14 days</b> — update your payment method to avoid interruption. Public &amp; personal Dōjōs are unaffected.</div>
            </div>
            <DojoBtn variant="danger" size="sm" style={{ flexShrink: 0 }}>Update payment</DojoBtn>
          </div>
        )}
        {/* current plan + seats */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-5)" }}>
          <div style={{ background: "var(--paper-soft)", border: "1px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
            <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-1)" }}>Current plan</div>
            <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, color: "var(--ink)", letterSpacing: "-0.01em" }}>Team · private</div>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", marginTop: "var(--space-1)", lineHeight: 1.5 }}>Renews Aug 1 · <span className="mono">${perSeat}</span>/active contributor/mo</div>
          </div>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
            <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-1)" }}>Billable seats</div>
            <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, color: "var(--ink)", lineHeight: 1 }}>{seatsActive}</div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-2)" }}>active contributors · <span style={{ color: "var(--success)" }}>{seatsReadonly} read-only free</span></div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)", lineHeight: 1.4 }}>Active = contributed or had a lesson attributed this period.</div>
          </div>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
            <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-1)" }}>This month</div>
            <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, color: "var(--ink)", lineHeight: 1 }}>${monthly}</div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-2)" }}>{seatsActive} × ${perSeat} · updated live</div>
          </div>
        </div>

        {/* tier compare */}
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-3)" }}>Tiers</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(250px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-5)" }}>
          {BILL_TIERS.map(t => (
            <div key={t.id} style={{ background: t.dark ? "var(--ink)" : "var(--paper-soft)", color: t.dark ? "var(--paper)" : "var(--ink)",
                  border: t.current ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-xl)", color: t.dark ? "var(--accent)" : t.current ? "var(--accent)" : "var(--ink-mute)" }}>{t.kanji}</span>
                <span style={{ fontSize: "var(--text-sm)", fontWeight: 600, color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.name}</span>
                {t.current && <span className="mono" style={{ fontSize: "var(--text-xs)", letterSpacing: ".06em", textTransform: "uppercase", color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)", marginLeft: "auto" }}>current</span>}
              </div>
              <div>
                <span className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 300, color: t.dark ? "var(--paper)" : "var(--ink)", letterSpacing: "-0.01em" }}>{t.price}</span>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: t.dark ? "var(--on-primary-mute)" : "var(--ink-faint)", marginLeft: "var(--space-1)" }}>{t.sub}</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
                {t.lines.map((l, i) => (
                  <div key={i} style={{ display: "flex", gap: "var(--space-2)", fontSize: "var(--text-xs)", lineHeight: 1.45, color: t.dark ? "var(--on-primary-soft)" : "var(--ink-soft)" }}>
                    <span style={{ color: "var(--accent)", flexShrink: 0 }}>·</span>{l}
                  </div>
                ))}
              </div>
              {!t.current && (
                <button style={{ marginTop: "auto", padding: "var(--space-2)", borderRadius: "var(--radius-lg)", border: t.dark ? "none" : "var(--hairline)", cursor: "pointer",
                      background: t.dark ? "var(--accent)" : "var(--paper)", color: t.dark ? "var(--paper)" : "var(--ink)", fontSize: "var(--text-sm)", fontFamily: "inherit", fontWeight: 500 }}>
                  {t.id === "free" ? "Downgrade to Free" : "Contact sales"}
                </button>
              )}
            </div>
          ))}
        </div>

        {/* Relay metering */}
        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", marginBottom: "var(--space-5)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>携</span>
            <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Relay — free for individuals, paid where it's shared</span>
          </div>
          <BillRelayRow label="Relay on your own projects — watch · approve · decide · nudge · chat" free tone="var(--success)" />
          <BillRelayRow label="One active machine · standard realtime · native-app push" free tone="var(--success)" />
          <BillRelayRow label="Shared team inbox & queue · presence (who's handling this)" tone="var(--accent)" />
          <BillRelayRow label="Higher concurrency · priority realtime · approval audit trail" tone="var(--accent)" />
          <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-2) 0" }}>
            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>Self-hosted relay · SSO on mobile · long-term approval-trail retention</span>
            <DojoChip tone="var(--ink-soft)">enterprise</DojoChip>
          </div>
        </div>

        {/* always free note */}
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-5)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--success)" }}>禅</span>
          <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
            <b style={{ fontWeight: 600, color: "var(--ink)" }}>Always free:</b> the desktop app, the global Collective, public &amp; personal Dōjōs, bring-your-own-key inference, and read-only membership. You pay only to coordinate a group's private knowledge — never for tokens.
          </span>
        </div>

        {/* seat roster + live Relay meters */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-5)" }}>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Billable seats</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>who's counted</span>
              <span style={{ flex: 1 }} />
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{seatsActive} active · {seatsReadonly} free</span>
            </div>
            <div>
              {[{ n: "Keiko T.", r: "shared 4 · attributed 11", on: true }, { n: "Marco D.", r: "shared 2 · attributed 6", on: true }, { n: "Sven K.", r: "read-only this period", on: false }, { n: "Mei L.", r: "shared 1 · attributed 3", on: true }].map((m, i, a) => (
                <div key={m.n} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-3) var(--space-4)", borderBottom: i < a.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <Avatar name={m.n} size={22} />
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{m.n}</div>
                    <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{m.r}</div>
                  </div>
                  {m.on ? <DojoChip tone="var(--accent)" soft="var(--accent-soft)">billable</DojoChip> : <DojoChip tone="var(--success)" soft="var(--success-soft)">free</DojoChip>}
                </div>
              ))}
              <div style={{ padding: "var(--space-2) var(--space-4)", fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>…30 more billable</div>
            </div>
          </div>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>携</span>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Relay · live this month</span>
            </div>
            <div style={{ padding: "var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              {[{ l: "Concurrent tracks · peak", v: "9", cap: "of 25", pct: 36 }, { l: "Shared inbox actions", v: "412", cap: "unlimited", pct: 60 }, { l: "Presence sessions", v: "28", cap: "of 40", pct: 70 }].map(x => (
                <div key={x.l}>
                  <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: "var(--space-1)" }}>
                    <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{x.l}</span>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{x.v} <span style={{ color: "var(--ink-faint)" }}>{x.cap}</span></span>
                  </div>
                  <div style={{ height: 5, borderRadius: "var(--radius-sm)", background: "var(--paper-mute)", overflow: "hidden" }}>
                    <div style={{ width: x.pct + "%", height: "100%", background: "var(--accent)", borderRadius: "var(--radius-sm)" }} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* invoices */}
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-2)" }}>Invoices</div>
        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
          {invoices.map((iv, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "1fr auto auto", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-3) var(--space-4)", borderBottom: i < invoices.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{iv.d}</span>
              <span className="mono" style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{iv.amt}</span>
              <DojoChip tone="var(--success)" soft="var(--success-soft)">{iv.s}</DojoChip>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

window.DojoBilling = DojoBilling;
