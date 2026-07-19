// Dōjō · in-app integration touchpoints (mocks).
// These are Observatory (desktop) screens — the moments where the app meets
// the company Dōjō: join, connect, bind, share, the global↔company toggle,
// and the downstream lane. Standalone mocks for review; wired later.
// Reuses globals from primitives.jsx (TauriChrome, Kanji, Avatar, StatusDot)
// and dojo-console.jsx (DojoChip, OriginChip).

const { useState: iaS } = React;

function InappFrame({ label, title, children, embedded }) {
  return (
    <div className="sensei" data-screen-label={label} style={{
      width: "100%", height: "100%", display: "flex", flexDirection: "column",
      background: "var(--paper)", overflow: "hidden",
    }}>
      {!embedded && <TauriChrome title={title} />}
      <div style={{ flex: 1, minHeight: 0, overflow: "auto", display: "flex", flexDirection: "column" }}>
        {children}
      </div>
    </div>
  );
}

function IaHead({ kanji, eyebrow, title, sub, right, mobile = false }) {
  return (
    <div style={{ display: "flex", alignItems: "flex-start", gap: mobile ? "var(--space-3)" : "var(--space-4)", flexWrap: mobile ? "wrap" : "nowrap",
                  padding: mobile ? "var(--space-4) var(--space-4) var(--space-3)" : "var(--space-5) var(--space-6) var(--space-4)",
                  borderBottom: "var(--hairline)", flexShrink: 0 }}>
      <span className="kanji" style={{ fontSize: mobile ? "var(--text-2xl)" : "var(--text-3xl)", color: "var(--accent)", lineHeight: 1, flexShrink: 0 }}>{kanji}</span>
      <div style={{ flex: 1, minWidth: mobile ? 180 : 0 }}>
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".18em", textTransform: "uppercase", color: "var(--ink-mute)", marginBottom: "var(--space-1)" }}>{eyebrow}</div>
        <h1 className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.05 }}>{title}</h1>
        {sub && <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, margin: "var(--space-1) 0 0", maxWidth: 700 }}>{sub}</p>}
      </div>
      {right && <div style={{ flexShrink: 0, width: mobile ? "100%" : "auto" }}>{right}</div>}
    </div>
  );
}

const btnPrimary = { background: "var(--ink)", color: "var(--paper)", border: "none", borderRadius: "var(--radius-lg)",
  padding: "var(--space-2) var(--space-4)", fontSize: "var(--text-sm)", fontWeight: 500, cursor: "pointer", fontFamily: "inherit",
  display: "inline-flex", alignItems: "center", gap: "var(--space-2)" };
const btnGhost = { background: "var(--paper-mute)", color: "var(--ink-soft)", border: "none", borderRadius: "var(--radius-lg)",
  padding: "var(--space-2) var(--space-4)", fontSize: "var(--text-sm)", cursor: "pointer", fontFamily: "inherit" };

/* ─── 1 · Bootstrap — join your org ──────────────────────── */
function InappJoin({ onContinue }) {
  return (
    <InappFrame label="First run · join your org" title="Sensei  先生  ·  first run">
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", padding: "var(--space-6)" }}>
        <div style={{ width: 560, background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)",
                      padding: "var(--space-6) var(--space-6)", textAlign: "center", boxShadow: "var(--shadow)" }}>
          <div style={{ position: "relative", height: 64, marginBottom: "var(--space-2)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-4xl)", color: "var(--accent)", lineHeight: 1 }}>結</span>
          </div>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-mute)", marginBottom: "var(--space-2)" }}>A Dōjō was detected</div>
          <h1 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: "0 0 var(--space-2)", lineHeight: 1.15 }}>
            <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>社</span> Acme Corp runs a Dōjō.
          </h1>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.6, margin: "0 0 var(--space-4)", maxWidth: 420, marginInline: "auto" }}>
            Join to inherit your team's standards on day one — and contribute what you learn back. Nothing here interrupts you; this card waits in Today and Preferences until you're ready.
          </p>
          {/* Resolution (Discover): pull, not push — multiple orgs, ranked by signal */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", margin: "0 0 var(--space-4)", textAlign: "left" }}>
            {[["社", "Acme Corp", "via SSO domain", "keiko@acme.com", true], ["客", "Globex", "via invite link", "engagement workspace", false]].map(([k, n, sig, detail, primary]) => (
              <div key={n} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-3) var(--space-3)", borderRadius: "var(--radius-lg)",
                            background: primary ? "var(--accent-soft)" : "var(--paper)",
                            border: primary ? "1px solid var(--accent-edge)" : "var(--hairline)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", width: 24, textAlign: "center" }}>{k}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", display: "flex", alignItems: "center", gap: "var(--space-2)" }}>{n}<DojoChip>{sig}</DojoChip></div>
                  <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{detail}</div>
                </div>
                <button onClick={onContinue} style={primary ? { ...btnPrimary, padding: "var(--space-2) var(--space-3)" } : { ...btnGhost, padding: "var(--space-2) var(--space-3)" }}>
                  {primary && <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>結</span>} Join
                </button>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", gap: "var(--space-2)", justifyContent: "center" }}>
            <button onClick={onContinue} style={btnGhost}>Not now · keep in Preferences</button>
          </div>
          <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-4)", lineHeight: 1.5 }}>
            Authenticated by SSO · nothing is shared until you choose to. Highest-confidence signal listed first.
          </div>
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 2 · Preferences — Connection pane ──────────────────── */
function InappConnection({ embedded }) {
  const memberships = [
    { k: "社", name: "Acme Corp", kind: "employer", scopes: "all your repos", on: true },
    { k: "客", name: "Globex", kind: "client", scopes: "lumen-auth · billing", on: true },
    { k: "客", name: "Initech", kind: "client", scopes: "initech-portal", on: true },
    { k: "群", name: "Rust Guild", kind: "community", scopes: "rust · tokio", on: false },
    { k: "己", name: "Personal", kind: "personal", scopes: "side projects", on: true },
  ];
  return (
    <InappFrame label="Preferences · Connection" title="Sensei  先生  ·  preferences" embedded={embedded}>
      <IaHead kanji="鍵" eyebrow="Preferences · connection" title="Connections"
        sub="Pair with a company-hosted Dōjō, authenticate, and choose which scopes you follow. You can belong to several at once."
        right={<button style={btnGhost}>+ Add connection</button>} />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-6)" }}>
        {/* connected server — Resolution (Authenticate): SSO = access, git = attribution */}
        <div style={{ background: "var(--paper-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", marginBottom: "var(--space-5)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>結</span>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>dojo.acme.internal</div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>session refreshes silently · device-code for the CLI</div>
            </div>
            <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--success)" }}>
              <StatusDot tone="success" /> connected
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-2)", marginTop: "var(--space-3)", paddingTop: "var(--space-3)", borderTop: "1px solid var(--paper-edge)" }}>
            {[["鍵", "Identity · access", "Work SSO", "keiko@acme.com"], ["署", "Attribution only", "Linked git", "github.com/keiko-t"]].map(([k, role, kind, who]) => (
              <div key={role} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", width: 20, textAlign: "center" }}>{k}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600 }}>{role}</div>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", marginTop: "var(--space-1)" }}>{kind} <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>· {who}</span></div>
                </div>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-3)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
            <span>SSO grants access; git only signs your attribution — it never grants access.</span>
            <span style={{ flex: 1 }} />
            <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer", whiteSpace: "nowrap" }}>Air-gapped? Use an offline token →</button>
          </div>
        </div>

        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-2)" }}>Your memberships</div>
        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
          {memberships.map((m, i) => (
            <div key={m.name} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto auto", gap: "var(--space-3)", alignItems: "center",
                          padding: "var(--space-3) var(--space-4)", borderBottom: i < memberships.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 22, textAlign: "center" }}>{m.k}</span>
              <div>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                  {m.name}<DojoChip>{m.kind}</DojoChip>
                </div>
                <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>following · {m.scopes}</div>
              </div>
              <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>scopes ▾</button>
              <span style={{ width: 36, height: 20, borderRadius: "var(--radius-full)", background: m.on ? "var(--ink)" : "var(--paper-mute)",
                            position: "relative", display: "inline-block", transition: "background .15s" }}>
                <span style={{ position: "absolute", top: 2, left: m.on ? 18 : 2, width: 16, height: 16, borderRadius: "50%", background: "var(--paper)", transition: "left .15s" }} />
              </span>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-3)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, maxWidth: 720 }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>客</span>
          <span>When a project belongs to a client engagement, that client connection takes precedence over your employer for what's shared and how it's attributed.</span>
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 3 · Project · About — bind to org ──────────────────── */
function InappBind() {
  return (
    <InappFrame label="Project · About — bind to org" title="Sensei  先生  ·  lumen-auth">
      <IaHead kanji="識" eyebrow="lumen-auth · about" title="Membership &amp; routing"
        sub="Bind this project to a connection so its findings route to the right Dōjō — and never pollute another. A project can span more than one client." />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-6)", maxWidth: 860 }}>
        {/* Resolution (Bind): explicit-confirm, multiple bindings, re-bind forward-only */}
        <div style={{ display: "flex", alignItems: "baseline", marginBottom: "var(--space-1)" }}>
          <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Bindings</span>
          <span style={{ flex: 1 }} />
          <button style={{ ...btnGhost, padding: "var(--space-1) var(--space-3)", fontSize: "var(--text-xs)" }}>+ Add binding</button>
        </div>
        <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, marginBottom: "var(--space-3)" }}>A default is inferred from each git remote and <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>confirmed at first scan</b> — never silently. Route different paths to different connections.</div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
          {/* confirmed binding */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper-soft)", border: "1px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>客</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>Client · Acme</div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>repo root · github.com/acme/lumen-auth</div>
            </div>
            <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ confirmed</DojoChip>
            <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none", cursor: "pointer" }}>change ▾</button>
          </div>
          {/* inferred binding awaiting confirm */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--ink-mute)" }}>客</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>Client · Globex</div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>path · /integrations/globex-sdk</div>
            </div>
            <DojoChip tone="var(--warning)" soft="var(--warning-soft)">inferred</DojoChip>
            <button style={{ ...btnPrimary, padding: "var(--space-2) var(--space-3)", fontSize: "var(--text-xs)" }}>Confirm</button>
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", margin: "var(--space-4) 0", padding: "var(--space-3) var(--space-4)",
                      background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-lg)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>盾</span>
          <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.55 }}>
            Findings route by the path they came from. Anything shared upstream from a client binding is <b style={{ fontWeight: 600 }}>anonymized</b> — the lesson travels, the source is dropped.
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>時</span>
          <span>Re-binding routes <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>future findings only</b> — past shares stay where they went, and history is never re-routed.</span>
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 4 · Project — ready to share lane ──────────────────── */
function InappShare({ embedded, mobile = false }) {
  const items = [
    { k: "守", title: "Validate webhook signatures before parsing", type: "Guard", origin: "client", scope: "Client · Acme", attrib: "anonymized", on: true, conf: 0.91 },
    { k: "問", title: "Integration-test persona for auth flows", type: "Prompt", origin: "employer", scope: "Stack · React", attrib: "Aiko N.", on: true, conf: 0.84 },
    { k: "紋", title: "Refresh-token rotation on device re-pair", type: "Pattern", origin: "client", scope: "Client · Acme", attrib: "anonymized", on: false, conf: 0.78 },
  ];
  const forming = { k: "芽", title: "Local fix for the staging seed script", reason: "too project-specific · hasn't generalised", conf: 0.41 };
  const [on, setOn] = iaS(() => items.map(i => i.on));
  const shareCount = on.filter(Boolean).length;
  return (
    <InappFrame label="Project · ready to share" title="Sensei  先生  ·  lumen-auth · memories" embedded={embedded}>
      <IaHead mobile={mobile} kanji="共" eyebrow="lumen-auth · memories" title="Ready to share"
        sub="Lessons generalised cleanly enough to leave this project. Pick a scope; attribution and anonymization are applied automatically by origin."
        right={<button style={btnPrimary}><span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>共</span> Share {shareCount} to Dōjō</button>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        {/* policy bar — Resolution (Share): org policy is the floor */}
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap", background: "var(--paper-soft)", border: "var(--hairline)",
                      borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-1)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>規</span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>Org floor</span>
          <DojoChip tone="var(--success)" soft="var(--success-soft)">always share · test personas</DojoChip>
          <DojoChip tone="var(--warning)" soft="var(--warning-soft)">never share · infra notes</DojoChip>
          <span style={{ flex: 1 }} />
          <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>edit policy</button>
        </div>
        <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginBottom: "var(--space-4)", lineHeight: 1.5 }}>Your org's policy is the floor — you can be stricter per item, never looser.</div>

        {/* Resolution (A finding forms): only past the bar surfaces here */}
        <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginBottom: "var(--space-2)", lineHeight: 1.5 }}>
          <span className="kanji" style={{ color: "var(--accent)", marginRight: "var(--space-1)" }}>芽</span>
          Only lessons past the <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>generalize + confidence bar</b> surface here — nothing is shareable by default.
        </div>

        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
          {items.map((it, i) => (
            <div key={i} onClick={() => setOn(a => a.map((v, k) => k === i ? !v : v))} style={{ display: "grid", gridTemplateColumns: mobile ? "auto auto 1fr" : "auto auto 1fr auto auto", gap: "var(--space-3)", alignItems: "center", cursor: "pointer",
                          padding: "var(--space-3) var(--space-4)", borderBottom: i < items.length - 1 ? "1px solid var(--paper-edge)" : "none", opacity: on[i] ? 1 : 0.55 }}>
              <span role="checkbox" aria-checked={on[i]} style={{ width: 18, height: 18, borderRadius: "var(--radius-sm)", border: "1.5px solid " + (on[i] ? "var(--accent)" : "var(--ink-faint)"),
                            background: on[i] ? "var(--accent)" : "transparent", color: "var(--paper)", fontSize: "var(--text-xs)", lineHeight: "15px", textAlign: "center" }}>{on[i] ? "✓" : ""}</span>
              <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 20, textAlign: "center" }}>{it.k}</span>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{it.title}</div>
                <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center", flexWrap: "wrap" }}>
                  <DojoChip>{it.type}</DojoChip><OriginChip origin={it.origin} />
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{it.attrib === "anonymized" ? "anonymized" : it.attrib}</span>
                  <DojoChip tone="var(--success)" soft="var(--success-soft)">generalised · {Math.round(it.conf * 100)}%</DojoChip>
                  {it.origin === "client" && <button onClick={e => e.stopPropagation()} className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0 }}>preview redaction →</button>}
                </div>
              </div>
              <button onClick={e => e.stopPropagation()} style={{ display: mobile ? "none" : "inline-flex", alignItems: "center", gap: "var(--space-1)", background: "var(--paper)", border: "var(--hairline)",
                            borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-2)", cursor: "pointer" }}>
                <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>{it.scope.startsWith("Client") ? "客" : "技"}</span>
                <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{it.scope}</span>
                <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>▾</span>
              </button>
              {!mobile && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>→ triage</span>}
            </div>
          ))}
        </div>
        {/* Resolution (A finding forms): below-the-bar items stay out of the lane */}
        <div style={{ marginTop: "var(--space-4)" }}>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-2)" }}>Still forming — below the bar</div>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper)", border: "1px dashed var(--ink-faint)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", opacity: 0.85 }}>
            <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--ink-faint)", width: 20, textAlign: "center" }}>{forming.k}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>{forming.title}</div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{forming.reason}</div>
            </div>
            <DojoChip>generalised · {Math.round(forming.conf * 100)}%</DojoChip>
          </div>
        </div>
        <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-4)", lineHeight: 1.5 }}>
          You'll see the redacted preview — raw vs dropped — before anything leaves. Shared items enter the org's triage queue; a maintainer approves before distribution, and you can recall a share until it's approved.
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 5b · Contributions — watch it travel ──────────────── */
// Resolution (Watch it travel): a status timeline per contribution, a decision
// notification with a reason on decline, and credit when adopted.
function InappTravel() {
  const steps = ["Queued", "Triaged", "Decided"];
  const contribs = [
    { k: "守", title: "Validate webhook signatures before parsing", scope: "Client · Acme", origin: "client", at: 2, outcome: "approved", note: "Approved by Keiko T. · adopted by 4 repos — credited to you internally.", when: "1d ago" },
    { k: "問", title: "Integration-test persona for auth flows", scope: "Stack · React", origin: "employer", at: 1, outcome: null, note: "In review with Sven K. · 2 ahead in the queue.", when: "3d ago" },
    { k: "紋", title: "Retry-on-429 for the billing client", scope: "Team · Payments", origin: "employer", at: 2, outcome: "declined", note: "Declined — duplicates an existing guard; merged into the canonical one.", when: "5d ago" },
  ];
  const Stat = ({ value, label }) => (
    <div style={{ flex: 1 }}>
      <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, color: "var(--ink)" }}>{value}</div>
      <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{label}</div>
    </div>
  );
  return (
    <InappFrame label="Contributions · watch it travel" title="Sensei  先生  ·  contributions">
      <IaHead kanji="旅" eyebrow="contributions" title="Where your shares went"
        sub="Every contribution carries a status timeline — and tells you the decision, with a reason on decline and credit when it's adopted. No silence after sharing." />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-6)" }}>
        <div style={{ display: "flex", gap: "var(--space-5)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", marginBottom: "var(--space-4)" }}>
          <Stat value="12" label="Shared · 30d" />
          <Stat value="9" label="Approved" />
          <Stat value="27" label="Repos adopting" />
          <Stat value="1" label="Declined" />
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          {contribs.map((c, idx) => {
            const decTone = c.outcome === "approved" ? "var(--success)" : c.outcome === "declined" ? "var(--danger)" : "var(--accent)";
            const decSoft = c.outcome === "approved" ? "var(--success-soft)" : c.outcome === "declined" ? "var(--danger-soft)" : "var(--accent-soft)";
            return (
            <div key={idx} style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
              <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "start" }}>
                <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>{c.k}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{c.title}</div>
                  <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center" }}>
                    <DojoChip>{c.scope}</DojoChip><OriginChip origin={c.origin} />
                  </div>
                </div>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{c.when}</span>
              </div>
              {/* status stepper */}
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", margin: "var(--space-3) 0 var(--space-3)", flexWrap: "wrap" }}>
                {steps.map((s, i) => {
                  const done = i <= c.at;
                  const isFinal = i === steps.length - 1;
                  const tone = isFinal && c.outcome ? decTone : "var(--accent)";
                  const soft = isFinal && c.outcome ? decSoft : "var(--accent-soft)";
                  const label = isFinal && c.outcome ? (c.outcome === "approved" ? "Approved" : "Declined") : s;
                  return (
                    <React.Fragment key={s}>
                      <span className="mono" style={{ fontSize: "var(--text-xs)", padding: "var(--space-1) var(--space-2)", borderRadius: "var(--radius-full)",
                                    color: done ? tone : "var(--ink-faint)", background: done ? soft : "var(--paper-mute)",
                                    border: done ? "1px solid transparent" : "var(--hairline)" }}>{done ? "✓ " : ""}{label}</span>
                      {i < steps.length - 1 && <span style={{ fontSize: "var(--text-xs)", color: i < c.at ? "var(--accent)" : "var(--ink-faint)" }}>→</span>}
                    </React.Fragment>
                  );
                })}
              </div>
              <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", paddingTop: "var(--space-3)", borderTop: "1px solid var(--paper-edge)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-sm)", color: decTone }}>{c.outcome === "declined" ? "返" : c.outcome === "approved" ? "果" : "待"}</span>
                <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.5, flex: 1 }}>{c.note}</span>
                {!c.outcome && (
                  <button style={{ flexShrink: 0, fontSize: "var(--text-xs)", color: "var(--ink-soft)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-3)", cursor: "pointer", fontFamily: "inherit", display: "inline-flex", alignItems: "center", gap: "var(--space-1)", whiteSpace: "nowrap" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>戻</span> Recall
                  </button>
                )}
              </div>
            </div>
            );
          })}
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 5 · Collective — global ↔ company toggle ───────────── */
function InappCollective() {
  const [src, setSrc] = iaS("dojo");
  const cats = [
    { name: "Patterns & rules", g: true, d: true },
    { name: "Guards & lints", g: true, d: true },
    { name: "Prompts & personas", g: false, d: true },
    { name: "Skills & agents", g: false, d: true },
  ];
  const isDojo = src === "dojo";
  return (
    <InappFrame label="Collective · global ↔ company" title="Sensei  先生  ·  collective intel">
      <IaHead kanji="群" eyebrow="collective intelligence" title="Where your knowledge flows"
        sub="Two destinations, each with its own controls — the public Collective, and your company Dōjō. They never share settings." />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-6)", maxWidth: 860 }}>
        {/* segmented toggle */}
        <div style={{ display: "inline-flex", background: "var(--paper-mute)", borderRadius: "var(--radius-lg)", padding: "var(--space-1)", marginBottom: "var(--space-5)" }}>
          {[["collective", "群 Public Collective"], ["dojo", "結 Acme Dōjō"]].map(([id, label]) => {
            const on = src === id;
            return (
              <button key={id} onClick={() => setSrc(id)} style={{
                padding: "var(--space-2) var(--space-4)", borderRadius: "var(--radius)", border: "none", cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-sm)",
                background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
                boxShadow: on ? "var(--shadow-sm)" : "none" }}>{label}</button>
            );
          })}
        </div>

        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-5)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-4)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>{isDojo ? "結" : "群"}</span>
            <div>
              <div style={{ fontSize: "var(--text-base)", color: "var(--ink)" }}>{isDojo ? "Acme Dōjō" : "Public Collective"}</div>
              <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{isDojo ? "private · governed · attributed to you internally" : "public commons · anonymized"}</div>
            </div>
            <span style={{ flex: 1 }} />
            <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>
              <StatusDot tone={isDojo ? "accent" : "ink-3"} /> sharing {isDojo ? "on · weekly" : "on · review first"}
            </span>
          </div>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-2)" }}>Categories shared {isDojo ? "to this Dōjō" : "publicly"}</div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
            {cats.map(c => {
              const on = isDojo ? c.d : c.g;
              return (
                <div key={c.name} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-2) 0", borderBottom: "1px solid var(--paper-edge)" }}>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1 }}>{c.name}</span>
                  {isDojo && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>scoped by binding</span>}
                  <span style={{ width: 36, height: 20, borderRadius: "var(--radius-full)", background: on ? "var(--ink)" : "var(--paper-mute)", position: "relative", display: "inline-block" }}>
                    <span style={{ position: "absolute", top: 2, left: on ? 18 : 2, width: 16, height: 16, borderRadius: "50%", background: "var(--paper)" }} />
                  </span>
                </div>
              );
            })}
          </div>
          {isDojo && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-3)", lineHeight: 1.5 }}>
              Client-bound projects are anonymized automatically before anything leaves. Separate cadence and filters from the public Collective.
            </div>
          )}
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 6 · Today / Upgrades — from your Dōjō ──────────────── */
function InappDownstream({ embedded }) {
  const iconBtn = { background: "var(--paper-mute)", border: "none", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-2)", cursor: "pointer", fontFamily: "var(--font-kanji)", fontSize: "var(--text-sm)", color: "var(--ink-mute)", lineHeight: 1 };
  const dojo = [
    { k: "守", title: "Never log refresh tokens, even at debug", scope: "Company", by: "approved by Keiko T.", impact: "+ prevents a known leak class", sup: true },
    { k: "紋", title: "Idempotency key on money-moving mutations", scope: "Team · Payments", by: "approved by Marco D.", impact: "+ matches 6 of your repos" },
  ];
  const global = [
    { k: "技", title: "Skill: explain a slow query plan", scope: "Postgres" },
  ];
  return (
    <InappFrame label="Upgrades · from your Dōjō" title="Sensei  先生  ·  upgrades" embedded={embedded}>
      <IaHead kanji="贈" eyebrow="upgrades" title="What's arrived for you"
        sub="Approved practice from your Dōjō lands here first — attributed and scoped to where you work — kept distinct from the public Collective. Mute or pin any scope; when rules conflict, the more specific scope wins." />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-6)" }}>
        {/* Resolution (Receive downstream): precedence ladder */}
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>序</span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>Precedence</span>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
            {["Org", "Team", "Global", "Personal"].map((s, i) => (
              <React.Fragment key={s}>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: i < 2 ? "var(--accent)" : "var(--ink-mute)" }}>{s}</span>
                {i < 3 && <span style={{ color: "var(--ink-faint)", fontSize: "var(--text-xs)" }}>›</span>}
              </React.Fragment>
            ))}
          </div>
          <span style={{ flex: 1 }} />
          <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>the more specific scope wins</span>
        </div>
        {/* Dōjō lane */}
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>結</span>
          <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", fontWeight: 600 }}>From your Dōjō · Acme</span>
          <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>2 new</span>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)", marginBottom: "var(--space-5)" }}>
          {dojo.map((u, i) => (
            <div key={i} style={{ background: "var(--paper-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)",
                          display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center" }}>
              <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>{u.k}</span>
              <div>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{u.title}</div>
                <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center", flexWrap: "wrap" }}>
                  <DojoChip tone="var(--accent)" soft="var(--accent-soft)">{u.scope}</DojoChip>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{u.by}</span>
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--success)" }}>{u.impact}</span>
                  {u.sup && <DojoChip tone="var(--ink-soft)">supersedes a Collective rule</DojoChip>}
                </div>
              </div>
              <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
                <button title="Pin this scope" style={iconBtn}>留</button>
                <button title="Mute this scope" style={iconBtn}>消</button>
                <button style={{ ...btnPrimary, padding: "var(--space-2) var(--space-3)" }}>Adopt</button>
                <button style={{ ...btnGhost, padding: "var(--space-2) var(--space-3)" }}>Defer</button>
              </div>
            </div>
          ))}
        </div>

        {/* Global lane */}
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--ink-mute)" }}>群</span>
          <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>From the Collective · public</span>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          {global.map((u, i) => (
            <div key={i} style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)",
                          display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center" }}>
              <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--ink-mute)" }}>{u.k}</span>
              <div>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{u.title}</div>
                <div style={{ marginTop: "var(--space-1)" }}><DojoChip>{u.scope}</DojoChip></div>
              </div>
              <div style={{ display: "flex", gap: "var(--space-2)" }}>
                <button style={{ ...btnGhost, padding: "var(--space-2) var(--space-3)" }}>Review</button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 4b · Redaction preview — raw vs dropped, before it leaves ── */
// The headline contributor promise made operational: never a blind send. For a
// client-origin finding, show exactly what travels (the lesson + what·why·impact
// + anonymized example) and exactly what is dropped (client · repo · identifiers
// · source), line by line, before the share is confirmed.
function InappRedact({ embedded, mobile = false }) {
  const dropped = [
    { k: "客", label: "Client & engagement", raw: "Acme Corp · billing-integration", tone: "var(--accent)" },
    { k: "庫", label: "Repo & paths", raw: "github.com/acme/lumen-auth · /webhooks/billing.ts", tone: "var(--accent)" },
    { k: "名", label: "Identifiers", raw: "ACME_WEBHOOK_SECRET · tenant_id=acme_prod", tone: "var(--accent)" },
    { k: "源", label: "Source reference", raw: "session s-2891 · commit 4f9c1a", tone: "var(--accent)" },
  ];
  const kept = [
    { k: "守", label: "The rule", v: "Validate the webhook HMAC signature before parsing the body." },
    { k: "憶", label: "What · why · impact", v: "Parsing before verifying let a forged payload reach the handler — a spoofed event mutated state. Verify first; reject on mismatch." },
    { k: "例", label: "Example · anonymized", v: "verify_signature(header, secret) → parse(body)  ·  identifiers dropped" },
  ];
  return (
    <InappFrame label="Project · redaction preview (raw vs dropped)" title="Sensei  先生  ·  lumen-auth" embedded={embedded}>
      <IaHead mobile={mobile} kanji="盾" eyebrow="Ready to share · client-origin" title="Here's exactly what leaves — and what doesn't"
        sub="This finding came from client work, so it's anonymized before anything leaves your machine. Review what's dropped below; nothing is sent until you confirm."
        right={<span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)", color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)" }}>盾 anonymized</span>} />

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4) var(--space-4) var(--space-2)" : "var(--space-5) var(--space-6) var(--space-2)" }}>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(300px, 1fr))", gap: "var(--space-4)", alignItems: "start" }}>
          {/* kept */}
          <div style={{ background: "var(--paper-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--success)", fontWeight: 700 }}>Travels upstream</span>
            </div>
            <div style={{ padding: "var(--space-1) var(--space-4) var(--space-3)" }}>
              {kept.map((x, i) => (
                <div key={i} style={{ padding: "var(--space-3) 0", borderBottom: i < kept.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--success)" }}>{x.k}</span>
                    <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{x.label}</span>
                  </div>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.55 }}>{x.v}</div>
                </div>
              ))}
            </div>
          </div>
          {/* dropped */}
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
              <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--ink-faint)" }} />
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 700 }}>Dropped — never leaves</span>
            </div>
            <div style={{ padding: "var(--space-1) var(--space-4) var(--space-3)" }}>
              {dropped.map((x, i) => (
                <div key={i} style={{ padding: "var(--space-3) 0", borderBottom: i < dropped.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>{x.k}</span>
                    <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{x.label}</span>
                    <span style={{ flex: 1 }} />
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textTransform: "uppercase", letterSpacing: ".06em" }}>dropped</span>
                  </div>
                  <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, textDecoration: "line-through", textDecorationColor: "var(--ink-faint)", wordBreak: "break-all" }}>{x.raw}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-4)", background: "var(--paper-soft)", border: "var(--hairline)", borderLeft: "3px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>盾</span>
          <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
            If a lesson can't stand without identifying context, it's <b style={{ fontWeight: 600, color: "var(--ink)" }}>dropped automatically</b> rather than weakened — never sent half-anonymized.
          </span>
        </div>
      </div>

      <div style={{ flexShrink: 0, borderTop: "var(--hairline)", padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-6)", display: "flex", alignItems: "center", gap: "var(--space-3)", flexWrap: "wrap" }}>
        {!mobile && <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>Destination · <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>Acme Corp Dōjō</b> · triage queue</span>}
        <span style={{ flex: 1 }} />
        <button style={btnGhost}>Cancel</button>
        <button style={btnPrimary}><span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>送</span> Confirm &amp; share anonymized</button>
      </div>
    </InappFrame>
  );
}

Object.assign(window, {
  InappJoin, InappConnection, InappBind, InappShare, InappRedact, InappTravel, InappCollective, InappDownstream,
});
