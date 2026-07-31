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
    <div className="grid gap-3 items-center py-2 px-0" style={{ gridTemplateColumns: "1fr auto", borderBottom: "1px solid var(--paper-edge)" }}>
      <span className="text-sm text-ink-soft" >{label}</span>
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="円" eyebrow="Org · plan & billing" title="Plan & billing"
        sub="Free where public or personal; paid where private and shared. Seats are billed per active contributor — read-only members, the desktop app, the global Collective and bring-your-own-key inference are always free."
        right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">Team · private</DojoChip>} />

      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        {past && (
          <div className="flex items-center gap-3 bg-danger-soft rounded-lg py-3 px-4 mb-4" style={{ flexWrap: mobile ? "wrap" : "nowrap", border: "1px solid var(--danger-edge)" }}>
            <span className="kanji text-lg text-danger" >滞</span>
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink font-semibold" >Payment past due · $408 unpaid</div>
              <div className="text-xs text-ink-soft mt-1" style={{ lineHeight: 1.5 }}>The Jul 1 charge failed. Private scopes stay active for <b className="font-semibold" >14 days</b> — update your payment method to avoid interruption. Public &amp; personal Dōjōs are unaffected.</div>
            </div>
            <DojoBtn variant="danger" size="sm" style={{ flexShrink: 0 }}>Update payment</DojoBtn>
          </div>
        )}
        {/* current plan + seats */}
        <div className="grid gap-3 mb-6" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))" }}>
          <div className="bg-paper-soft rounded-lg py-4 px-4" style={{ border: "1px solid var(--accent)" }}>
            <div className="text-xs uppercase text-ink-mute font-semibold mb-1" style={{ letterSpacing: ".12em" }}>Current plan</div>
            <div className="display text-2xl font-light text-ink" style={{ letterSpacing: "-0.01em" }}>Team · private</div>
            <div className="text-sm text-ink-soft mt-1" style={{ lineHeight: 1.5 }}>Renews Aug 1 · <span className="mono">${perSeat}</span>/active contributor/mo</div>
          </div>
          <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" >
            <div className="text-xs uppercase text-ink-mute font-semibold mb-1" style={{ letterSpacing: ".12em" }}>Billable seats</div>
            <div className="display text-2xl font-light text-ink" style={{ lineHeight: 1 }}>{seatsActive}</div>
            <div className="text-xs text-ink-mute mt-2" >active contributors · <span className="text-success" >{seatsReadonly} read-only free</span></div>
            <div className="text-xs text-ink-faint mt-1" style={{ lineHeight: 1.4 }}>Active = contributed or had a lesson attributed this period.</div>
          </div>
          <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" >
            <div className="text-xs uppercase text-ink-mute font-semibold mb-1" style={{ letterSpacing: ".12em" }}>This month</div>
            <div className="display text-2xl font-light text-ink" style={{ lineHeight: 1 }}>${monthly}</div>
            <div className="text-xs text-ink-mute mt-2" >{seatsActive} × ${perSeat} · updated live</div>
          </div>
        </div>

        {/* tier compare */}
        <div className="text-xs uppercase text-ink-mute font-semibold mb-3" style={{ letterSpacing: ".14em" }}>Tiers</div>
        <div className="grid gap-3 mb-6" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(250px, 1fr))" }}>
          {BILL_TIERS.map(t => (
            <div className="rounded-lg py-4 px-4 flex flex-col gap-2" key={t.id} style={{ background: t.dark ? "var(--ink)" : "var(--paper-soft)", color: t.dark ? "var(--paper)" : "var(--ink)",
 border: t.current ? "1px solid var(--accent)" : "var(--hairline)" }}>
              <div className="flex items-center gap-2" >
                <span className="kanji text-xl" style={{ color: t.dark ? "var(--accent)" : t.current ? "var(--accent)" : "var(--ink-mute)" }}>{t.kanji}</span>
                <span className="text-sm font-semibold" style={{ color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.name}</span>
                {t.current && <span className="mono text-xs uppercase text-accent bg-accent-soft rounded-full py-1 px-2" style={{ letterSpacing: ".06em", border: "1px solid var(--accent-edge)", marginLeft: "auto" }}>current</span>}
              </div>
              <div>
                <span className="display text-xl font-light" style={{ color: t.dark ? "var(--paper)" : "var(--ink)", letterSpacing: "-0.01em" }}>{t.price}</span>
                <span className="mono text-xs ml-1" style={{ color: t.dark ? "var(--on-primary-mute)" : "var(--ink-faint)" }}>{t.sub}</span>
              </div>
              <div className="flex flex-col gap-1" >
                {t.lines.map((l, i) => (
                  <div className="flex gap-2 text-xs" key={i} style={{ lineHeight: 1.45, color: t.dark ? "var(--on-primary-soft)" : "var(--ink-soft)" }}>
                    <span className="text-accent shrink-0" >·</span>{l}
                  </div>
                ))}
              </div>
              {!t.current && (
                <button className="p-2 rounded-lg cursor-pointer text-sm font-medium" style={{ marginTop: "auto", border: t.dark ? "none" : "var(--hairline)",
 background: t.dark ? "var(--accent)" : "var(--paper)", color: t.dark ? "var(--paper)" : "var(--ink)", fontFamily: "inherit" }}>
                  {t.id === "free" ? "Downgrade to Free" : "Contact sales"}
                </button>
              )}
            </div>
          ))}
        </div>

        {/* Relay metering */}
        <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4 mb-6" >
          <div className="flex items-center gap-2 mb-2" >
            <span className="kanji text-base text-accent" >携</span>
            <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Relay — free for individuals, paid where it's shared</span>
          </div>
          <BillRelayRow label="Relay on your own projects — watch · approve · decide · nudge · chat" free tone="var(--success)" />
          <BillRelayRow label="One active machine · standard realtime · native-app push" free tone="var(--success)" />
          <BillRelayRow label="Shared team inbox & queue · presence (who's handling this)" tone="var(--accent)" />
          <BillRelayRow label="Higher concurrency · priority realtime · approval audit trail" tone="var(--accent)" />
          <div className="grid gap-3 items-center py-2 px-0" style={{ gridTemplateColumns: "1fr auto" }}>
            <span className="text-sm text-ink-soft" >Self-hosted relay · SSO on mobile · long-term approval-trail retention</span>
            <DojoChip tone="var(--ink-soft)">enterprise</DojoChip>
          </div>
        </div>

        {/* always free note */}
        <div className="flex items-start gap-2 bg-success-soft rounded-lg py-3 px-4 mb-6" style={{ border: "1px solid var(--success-edge)" }}>
          <span className="kanji text-base text-success" >禅</span>
          <span className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>
            <b className="font-semibold text-ink" >Always free:</b> the desktop app, the global Collective, public &amp; personal Dōjōs, bring-your-own-key inference, and read-only membership. You pay only to coordinate a group's private knowledge — never for tokens.
          </span>
        </div>

        {/* seat roster + live Relay meters */}
        <div className="grid gap-3 mb-6" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))" }}>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            <div className="flex items-center gap-2 py-3 px-4 border-b" >
              <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Billable seats</span>
              <span className="mono text-xs text-ink-faint" >who's counted</span>
              <span className="flex-1" />
              <span className="mono text-xs text-ink-faint" >{seatsActive} active · {seatsReadonly} free</span>
            </div>
            <div>
              {[{ n: "Keiko T.", r: "shared 4 · attributed 11", on: true }, { n: "Marco D.", r: "shared 2 · attributed 6", on: true }, { n: "Sven K.", r: "read-only this period", on: false }, { n: "Mei L.", r: "shared 1 · attributed 3", on: true }].map((m, i, a) => (
                <div className="grid gap-3 items-center py-3 px-4" key={m.n} style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: i < a.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <Avatar name={m.n} size={22} />
                  <div className="min-w-0" >
                    <div className="text-sm text-ink" >{m.n}</div>
                    <div className="mono text-xs text-ink-faint mt-1" >{m.r}</div>
                  </div>
                  {m.on ? <DojoChip tone="var(--accent)" soft="var(--accent-soft)">billable</DojoChip> : <DojoChip tone="var(--success)" soft="var(--success-soft)">free</DojoChip>}
                </div>
              ))}
              <div className="py-2 px-4 text-xs text-ink-faint" >…30 more billable</div>
            </div>
          </div>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            <div className="flex items-center gap-2 py-3 px-4 border-b" >
              <span className="kanji text-sm text-accent" >携</span>
              <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Relay · live this month</span>
            </div>
            <div className="p-4 flex flex-col gap-3" >
              {[{ l: "Concurrent tracks · peak", v: "9", cap: "of 25", pct: 36 }, { l: "Shared inbox actions", v: "412", cap: "unlimited", pct: 60 }, { l: "Presence sessions", v: "28", cap: "of 40", pct: 70 }].map(x => (
                <div key={x.l}>
                  <div className="flex items-baseline justify-between mb-1" >
                    <span className="text-xs text-ink-soft" >{x.l}</span>
                    <span className="mono text-xs text-ink-mute" >{x.v} <span className="text-ink-faint" >{x.cap}</span></span>
                  </div>
                  <div className="rounded-sm bg-paper-mute overflow-hidden" style={{ height: 5 }}>
                    <div className="h-full bg-accent rounded-sm" style={{ width: x.pct + "%" }} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* invoices */}
        <div className="text-xs uppercase text-ink-mute font-semibold mb-2" style={{ letterSpacing: ".14em" }}>Invoices</div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {invoices.map((iv, i) => (
            <div className="grid gap-3 items-center py-3 px-4" key={i} style={{ gridTemplateColumns: "1fr auto auto", borderBottom: i < invoices.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <span className="mono text-xs text-ink-soft" >{iv.d}</span>
              <span className="mono text-sm text-ink" >{iv.amt}</span>
              <DojoChip tone="var(--success)" soft="var(--success-soft)">{iv.s}</DojoChip>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

window.DojoBilling = DojoBilling;
