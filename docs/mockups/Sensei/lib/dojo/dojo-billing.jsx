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
    <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 12, alignItems: "center", padding: "9px 0", borderBottom: "1px solid var(--edge)" }}>
      <span style={{ fontSize: 12.5, color: "var(--ink-2)" }}>{label}</span>
      <DojoChip tone={tone} soft={free ? "var(--success-soft)" : "var(--accent-soft)"}>{free ? "free · individuals" : "paid · team"}</DojoChip>
    </div>
  );
}

function DojoBilling() {
  const seatsActive = 34, seatsReadonly = 14, perSeat = 12;
  const monthly = seatsActive * perSeat;
  const invoices = [
    { d: "Jul 1, 2026", amt: `$${monthly}.00`, s: "paid" },
    { d: "Jun 1, 2026", amt: "$396.00", s: "paid" },
    { d: "May 1, 2026", amt: "$372.00", s: "paid" },
  ];
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="円" eyebrow="Org · plan & billing" title="Plan & billing"
        sub="Free where public or personal; paid where private and shared. Seats are billed per active contributor — read-only members, the desktop app, the global Collective and bring-your-own-key inference are always free."
        right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">Team · private</DojoChip>} />

      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        {/* current plan + seats */}
        <div style={{ display: "grid", gridTemplateColumns: "1.3fr 1fr 1fr", gap: 14, marginBottom: 24 }}>
          <div style={{ background: "var(--paper-2)", border: "1px solid var(--accent)", borderRadius: 12, padding: "16px 18px" }}>
            <div style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 6 }}>Current plan</div>
            <div className="display" style={{ fontSize: 26, fontWeight: 300, color: "var(--ink)", letterSpacing: "-0.01em" }}>Team · private</div>
            <div style={{ fontSize: 12.5, color: "var(--ink-2)", marginTop: 6, lineHeight: 1.5 }}>Renews Aug 1 · <span className="mono">${perSeat}</span>/active contributor/mo</div>
          </div>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px" }}>
            <div style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 6 }}>Billable seats</div>
            <div className="display" style={{ fontSize: 34, fontWeight: 300, color: "var(--ink)", lineHeight: 1 }}>{seatsActive}</div>
            <div style={{ fontSize: 12, color: "var(--ink-3)", marginTop: 7 }}>active contributors · <span style={{ color: "var(--success)" }}>{seatsReadonly} read-only free</span></div>
          </div>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px" }}>
            <div style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 6 }}>This month</div>
            <div className="display" style={{ fontSize: 34, fontWeight: 300, color: "var(--ink)", lineHeight: 1 }}>${monthly}</div>
            <div style={{ fontSize: 12, color: "var(--ink-3)", marginTop: 7 }}>{seatsActive} × ${perSeat} · updated live</div>
          </div>
        </div>

        {/* tier compare */}
        <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 12 }}>Tiers</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 14, marginBottom: 26 }}>
          {BILL_TIERS.map(t => (
            <div key={t.id} style={{ background: t.dark ? "var(--ink)" : "var(--paper-2)", color: t.dark ? "var(--paper)" : "var(--ink)",
                  border: t.current ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: 12, padding: "16px 18px", display: "flex", flexDirection: "column", gap: 10 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span className="kanji" style={{ fontSize: 20, color: t.dark ? "var(--accent)" : t.current ? "var(--accent)" : "var(--ink-3)" }}>{t.kanji}</span>
                <span style={{ fontSize: 14, fontWeight: 600, color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.name}</span>
                {t.current && <span className="mono" style={{ fontSize: 9, letterSpacing: ".06em", textTransform: "uppercase", color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid oklch(0.58 0.15 35/.28)", borderRadius: 20, padding: "2px 7px", marginLeft: "auto" }}>current</span>}
              </div>
              <div>
                <span className="display" style={{ fontSize: 22, fontWeight: 300, color: t.dark ? "var(--paper)" : "var(--ink)", letterSpacing: "-0.01em" }}>{t.price}</span>
                <span className="mono" style={{ fontSize: 10.5, color: t.dark ? "color-mix(in oklch, var(--paper) 62%, transparent)" : "var(--ink-4)", marginLeft: 6 }}>{t.sub}</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {t.lines.map((l, i) => (
                  <div key={i} style={{ display: "flex", gap: 7, fontSize: 11.5, lineHeight: 1.45, color: t.dark ? "color-mix(in oklch, var(--paper) 74%, transparent)" : "var(--ink-2)" }}>
                    <span style={{ color: "var(--accent)", flexShrink: 0 }}>·</span>{l}
                  </div>
                ))}
              </div>
              {!t.current && (
                <button style={{ marginTop: "auto", padding: "9px", borderRadius: 8, border: t.dark ? "none" : "var(--hairline)", cursor: "pointer",
                      background: t.dark ? "var(--accent)" : "var(--paper)", color: t.dark ? "var(--paper)" : "var(--ink)", fontSize: 12.5, fontFamily: "inherit", fontWeight: 500 }}>
                  {t.id === "free" ? "Downgrade to Free" : "Contact sales"}
                </button>
              )}
            </div>
          ))}
        </div>

        {/* Relay metering */}
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px", marginBottom: 24 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 9, marginBottom: 10 }}>
            <span className="kanji" style={{ fontSize: 16, color: "var(--accent)" }}>携</span>
            <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Relay — free for individuals, paid where it's shared</span>
          </div>
          <BillRelayRow label="Relay on your own projects — watch · approve · decide · nudge · chat" free tone="var(--success)" />
          <BillRelayRow label="One active machine · standard realtime · native-app push" free tone="var(--success)" />
          <BillRelayRow label="Shared team inbox & queue · presence (who's handling this)" tone="var(--accent)" />
          <BillRelayRow label="Higher concurrency · priority realtime · approval audit trail" tone="var(--accent)" />
          <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 12, alignItems: "center", padding: "9px 0" }}>
            <span style={{ fontSize: 12.5, color: "var(--ink-2)" }}>Self-hosted relay · SSO on mobile · long-term approval-trail retention</span>
            <DojoChip tone="var(--ink-2)">enterprise</DojoChip>
          </div>
        </div>

        {/* always free note */}
        <div style={{ display: "flex", alignItems: "flex-start", gap: 10, background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: 12, padding: "14px 16px", marginBottom: 24 }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--success)" }}>禅</span>
          <span style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
            <b style={{ fontWeight: 600, color: "var(--ink)" }}>Always free:</b> the desktop app, the global Collective, public &amp; personal Dōjōs, bring-your-own-key inference, and read-only membership. You pay only to coordinate a group's private knowledge — never for tokens.
          </span>
        </div>

        {/* invoices */}
        <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 10 }}>Invoices</div>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
          {invoices.map((iv, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "1fr auto auto", gap: 14, alignItems: "center", padding: "12px 16px", borderBottom: i < invoices.length - 1 ? "1px solid var(--edge)" : "none" }}>
              <span className="mono" style={{ fontSize: 12, color: "var(--ink-2)" }}>{iv.d}</span>
              <span className="mono" style={{ fontSize: 12.5, color: "var(--ink)" }}>{iv.amt}</span>
              <DojoChip tone="var(--success)" soft="var(--success-soft)">{iv.s}</DojoChip>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

window.DojoBilling = DojoBilling;
