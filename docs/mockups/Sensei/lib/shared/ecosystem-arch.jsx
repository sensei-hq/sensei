// The Sensei ecosystem — one architecture board showing every component and
// how they connect: the local-first desktop app on the developer's machine
// (CLI · MCP · daemon · DB · embedded Ollama), the inference gateway routing to
// external Ollama and BYOK cloud models, the SaaS/self-hosted Dōjō service+web,
// and the mobile/pad Relay companion. Token-only → theme-free.

function EcoNode({ k, title, sub, tone = "var(--accent)", solid, children, style }) {
  return (
    <div style={{ background: solid ? "var(--paper-3)" : "var(--paper-2)", border: "var(--hairline)",
      borderRadius: 10, padding: "12px 14px", display: "flex", flexDirection: "column", gap: 6, ...style }}>
      <div style={{ display: "flex", alignItems: "center", gap: 9 }}>
        <span className="kanji" style={{ fontSize: 20, color: tone, lineHeight: 1, flexShrink: 0 }}>{k}</span>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 13.5, color: "var(--ink)", fontWeight: 500 }}>{title}</div>
          {sub && <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)", marginTop: 1 }}>{sub}</div>}
        </div>
      </div>
      {children}
    </div>
  );
}

function EcoZone({ label, kanji, note, accent, children, style }) {
  return (
    <div style={{ position: "relative", border: `1.5px dashed color-mix(in oklch, ${accent} 45%, var(--edge))`,
      borderRadius: 16, padding: "26px 18px 18px", background: `color-mix(in oklch, ${accent} 4%, transparent)`, ...style }}>
      <div style={{ position: "absolute", top: -11, left: 18, display: "inline-flex", alignItems: "center", gap: 7,
        background: "var(--paper)", padding: "0 9px" }}>
        <span className="kanji" style={{ fontSize: 15, color: accent }}>{kanji}</span>
        <span style={{ fontSize: 10.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{label}</span>
        {note && <span style={{ fontSize: 10.5, color: "var(--ink-4)" }}>· {note}</span>}
      </div>
      {children}
    </div>
  );
}

function EcoLink({ label, sub, dir = "→" }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 4, padding: "0 6px", minWidth: 96 }}>
      <div style={{ fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, textAlign: "center" }}>{label}</div>
      <div style={{ fontSize: 20, color: "var(--accent)", lineHeight: 1 }}>{dir}</div>
      {sub && <div className="mono" style={{ fontSize: 9.5, color: "var(--ink-4)", textAlign: "center" }}>{sub}</div>}
    </div>
  );
}

function EcosystemArchitecture() {
  const byok = ["Claude", "GPT-4o", "Qwen 2.5", "Kimi K2", "…your keys"];
  return (
    <div className="sensei" data-screen-label="Ecosystem architecture" data-om-raster
         style={{ width: 1600, minHeight: 980, background: "var(--paper)", padding: 48, display: "flex", flexDirection: "column", gap: 8, boxSizing: "border-box" }}>
      {/* title */}
      <div style={{ display: "flex", alignItems: "flex-start", gap: 16, marginBottom: 8 }}>
        <span className="kanji" style={{ fontSize: 44, color: "var(--accent)", lineHeight: 1 }}>系</span>
        <div>
          <div style={{ fontSize: 11, letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-3)" }}>Sensei · system architecture</div>
          <h1 className="display" style={{ fontSize: 34, fontWeight: 300, letterSpacing: "-0.02em", margin: "4px 0 0", color: "var(--ink)" }}>The ecosystem, end to end</h1>
          <p style={{ fontSize: 14, color: "var(--ink-2)", lineHeight: 1.6, margin: "8px 0 0", maxWidth: 900 }}>
            Local-first on the developer's machine; inference routed through one gateway to the models you bring;
            knowledge shared to a Dōjō you host or subscribe to; a phone/pad companion for the human-in-the-loop moments.
          </p>
        </div>
      </div>

      <div style={{ display: "flex", gap: 4, alignItems: "stretch", flex: 1 }}>
        {/* ── Developer machine ── */}
        <EcoZone label="On the developer's machine" kanji="机" note="local-first" accent="var(--ink)"
          style={{ flex: "0 0 500px", display: "flex", flexDirection: "column", gap: 16 }}>
          <EcoNode k="観" title="Sensei · desktop app" sub="Tauri · observes every session" tone="var(--accent)">
            <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5, marginBottom: 4 }}>Wraps the local runtime — your code and sessions never leave unless you share.</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <EcoNode k="令" title="CLI" sub="sensei run" tone="var(--ink-3)" solid />
              <EcoNode k="具" title="MCP servers" sub="tools · instruments" tone="var(--ink-3)" solid />
              <EcoNode k="常" title="daemon" sub="always-on watcher" tone="var(--ink-3)" solid />
              <EcoNode k="蔵" title="Postgres DB" sub="local store" tone="var(--ink-3)" solid />
              <EcoNode k="火" title="Embedded Ollama" sub="bundled local models" tone="var(--ink-3)" solid style={{ gridColumn: "1 / -1" }} />
            </div>
          </EcoNode>
          <EcoNode k="携" title="Relay · mobile / pad" sub="the away-from-keyboard companion" tone="var(--success)">
            <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5 }}>
              Pairs with the daemon over an <b style={{ fontWeight: 600, color: "var(--ink)" }}>encrypted channel</b> — approve gated actions, answer decisions, watch progress while away.
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 4, fontSize: 10.5, color: "var(--ink-3)" }}>
              <span style={{ fontSize: 14, color: "var(--ink-4)" }}>↑↓</span> encrypted pairing · to the daemon
            </div>
          </EcoNode>
        </EcoZone>

        {/* links from machine */}
        <div style={{ display: "flex", flexDirection: "column", justifyContent: "space-around" }}>
          <EcoLink label="inference" sub="local IPC → HTTPS" />
          <EcoLink label="knowledge" sub="HTTPS · share / pull" />
        </div>

        {/* ── right stack: inference + cloud ── */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 16, minWidth: 0 }}>
          {/* Inference */}
          <EcoZone label="Inference" kanji="門" note="one gateway · your keys" accent="var(--accent)"
            style={{ display: "flex", alignItems: "stretch", gap: 4 }}>
            <EcoNode k="門" title="Gateway" sub="BYOK · routing + actions" tone="var(--accent)" style={{ flex: "0 0 220px" }}>
              <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5 }}>One entry point; picks the right model per task and carries your own API keys.</div>
            </EcoNode>
            <EcoLink label="routes to" />
            <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 10, minWidth: 0 }}>
              <EcoNode k="火" title="External Ollama" sub="self-hosted · your GPUs" tone="var(--ink-2)" />
              <EcoNode k="雲" title="BYOK cloud models" sub="bring your own keys" tone="var(--ink-2)">
                <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 2 }}>
                  {byok.map(m => (
                    <span key={m} className="mono" style={{ fontSize: 10.5, color: "var(--ink-2)", background: "var(--paper-3)",
                      border: "var(--hairline)", borderRadius: 20, padding: "2px 9px" }}>{m}</span>
                  ))}
                </div>
              </EcoNode>
            </div>
          </EcoZone>

          {/* Cloud / team */}
          <EcoZone label="Team & cloud" kanji="結" note="self-hosted or SaaS" accent="var(--success)"
            style={{ flex: 1, display: "flex", alignItems: "stretch", gap: 14 }}>
            <EcoNode k="結" title="Dōjō · service + web" sub="dojo.sensei-hq.com · or your VPC" tone="var(--success)" style={{ flex: 1 }}>
              <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5, marginBottom: 6 }}>
                The company hive-mind: contribute → triage → approve → distribute. Every login lands here; teams & orgs get the governance.
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                <span className="mono" style={{ fontSize: 10, color: "var(--success)", background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: 20, padding: "2px 9px" }}>self-host · in-house</span>
                <span className="mono" style={{ fontSize: 10, color: "var(--ink-2)", background: "var(--paper-3)", border: "var(--hairline)", borderRadius: 20, padding: "2px 9px" }}>or managed SaaS</span>
              </div>
            </EcoNode>
            <EcoNode k="群" title="Global Collective" sub="public commons · anonymized" tone="var(--ink-2)" style={{ flex: "0 0 240px" }}>
              <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5 }}>Opt-in public knowledge across all Sensei users, separate from any Dōjō.</div>
            </EcoNode>
          </EcoZone>
        </div>
      </div>

      {/* legend */}
      <div style={{ display: "flex", alignItems: "center", gap: 20, marginTop: 12, paddingTop: 14, borderTop: "var(--hairline)", flexWrap: "wrap" }}>
        <span style={{ fontSize: 10.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600 }}>Boundaries</span>
        {[["机", "Your machine — code & sessions stay local", "var(--ink)"],
          ["門", "Inference — your keys, your models", "var(--accent)"],
          ["結", "Dōjō — hosted by you or as SaaS", "var(--success)"],
          ["群", "Collective — public, opt-in", "var(--ink-2)"]].map(([k, t, c]) => (
          <span key={t} style={{ display: "inline-flex", alignItems: "center", gap: 7, fontSize: 12, color: "var(--ink-2)" }}>
            <span className="kanji" style={{ fontSize: 14, color: c }}>{k}</span>{t}
          </span>
        ))}
      </div>
    </div>
  );
}

window.EcosystemArchitecture = EcosystemArchitecture;
