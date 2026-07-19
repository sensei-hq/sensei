// The Sensei ecosystem — one architecture board showing every component and
// how they connect. Local-first desktop app on the developer's machine
// (CLI · MCP · daemon · Postgres). The Sensei Gateway is a referenced library
// INSIDE the desktop app: it embeds Ollama for fast inference (defaults to
// Gemma 4) and can also route to a local/external Ollama service or BYOK cloud
// models. Knowledge is two-way — individuals push/publish up to a Dōjō, and
// consolidated knowledge flows back down from the team/org Dōjō or the global
// collective. A mobile/pad Relay companion pairs with the daemon.
// Token-only → theme-free.

function EcoNode({ k, title, sub, tone = "var(--accent)", solid, children, style }) {
  return (
    <div style={{ background: solid ? "var(--paper-3)" : "var(--paper-2)", border: "var(--hairline)",
      borderRadius: 10, padding: "12px 16px", display: "flex", flexDirection: "column", gap: 6, ...style }}>
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
      borderRadius: 16, padding: "24px 16px 16px", background: `color-mix(in oklch, ${accent} 4%, transparent)`, ...style }}>
      <div style={{ position: "absolute", top: -11, left: 18, display: "inline-flex", alignItems: "center", gap: 7,
        background: "var(--paper)", padding: "0 12px" }}>
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
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 4, padding: "0 4px", minWidth: 92 }}>
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
         style={{ width: 1600, minHeight: 1000, background: "var(--paper)", padding: 48, display: "flex", flexDirection: "column", gap: 8, boxSizing: "border-box" }}>
      {/* title */}
      <div style={{ display: "flex", alignItems: "flex-start", gap: 16, marginBottom: 8 }}>
        <span className="kanji" style={{ fontSize: 44, color: "var(--accent)", lineHeight: 1 }}>系</span>
        <div>
          <div style={{ fontSize: 11, letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-3)" }}>Sensei · system architecture</div>
          <h1 className="display" style={{ fontSize: 34, fontWeight: 300, letterSpacing: "-0.02em", margin: "4px 0 0", color: "var(--ink)" }}>The ecosystem, end to end</h1>
          <p style={{ fontSize: 14, color: "var(--ink-2)", lineHeight: 1.6, margin: "8px 0 0", maxWidth: 940 }}>
            Local-first on the developer's machine; inference through one embedded gateway — fast by default, your keys when you want them;
            knowledge shared <b style={{ fontWeight: 600, color: "var(--ink)" }}>both ways</b> with a Dōjō you host or subscribe to; a phone/pad companion for the human-in-the-loop moments.
          </p>
        </div>
      </div>

      <div style={{ display: "flex", gap: 4, alignItems: "stretch", flex: 1 }}>
        {/* ── Developer machine ── */}
        <EcoZone label="On the developer's machine" kanji="机" note="local-first" accent="var(--ink)"
          style={{ flex: "0 0 560px", display: "flex", flexDirection: "column", gap: 16 }}>
          <EcoNode k="観" title="Sensei · desktop app" sub="Tauri · observes every session" tone="var(--accent)">
            <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5, marginBottom: 4 }}>Wraps the local runtime — your code and sessions never leave unless you share.</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <EcoNode k="令" title="CLI" sub="sensei run" tone="var(--ink-3)" solid />
              <EcoNode k="具" title="MCP servers" sub="tools · instruments" tone="var(--ink-3)" solid />
              <EcoNode k="常" title="daemon" sub="always-on watcher" tone="var(--ink-3)" solid />
              <EcoNode k="蔵" title="Postgres DB" sub="local store" tone="var(--ink-3)" solid />
            </div>
            {/* Gateway — a referenced library inside the app, embeds Ollama */}
            <EcoNode k="門" title="Sensei Gateway" sub="referenced library · in-app" tone="var(--accent)"
              style={{ marginTop: 2, border: "1px solid color-mix(in oklch, var(--accent) 40%, var(--edge))" }}>
              <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5 }}>
                One inference entry point, linked into the desktop app. Picks the model per task and carries your keys.
              </div>
              <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: 8, padding: "12px 12px", marginTop: 4 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span className="kanji" style={{ fontSize: 16, color: "var(--accent)" }}>火</span>
                  <span style={{ fontSize: 12.5, color: "var(--ink)", fontWeight: 500, flex: 1 }}>Embedded Ollama</span>
                  <span className="mono" style={{ fontSize: 9.5, color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: 20, padding: "4px 8px" }}>default · Gemma 4</span>
                </div>
                <div style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 5 }}>bundled · fast local inference, no setup</div>
              </div>
            </EcoNode>
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
          <EcoLink label="inference" sub="gateway → models" dir="⇄" />
          <EcoLink label="knowledge" sub="push ↑ · consolidated ↓" dir="⇄" />
        </div>

        {/* ── right stack: inference + cloud ── */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 16, minWidth: 0 }}>
          {/* Inference targets the gateway can also use */}
          <EcoZone label="Other inference the gateway can use" kanji="択" note="optional · BYOK" accent="var(--accent)"
            style={{ display: "flex", alignItems: "stretch", gap: 12 }}>
            <EcoNode k="火" title="Ollama service" sub="local or external · your GPUs" tone="var(--ink-2)" style={{ flex: 1 }}>
              <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5 }}>Point the gateway at a bigger Ollama host when the embedded one isn't enough.</div>
            </EcoNode>
            <EcoNode k="雲" title="BYOK cloud models" sub="bring your own keys" tone="var(--ink-2)" style={{ flex: 1 }}>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 2 }}>
                {byok.map(m => (
                  <span key={m} className="mono" style={{ fontSize: 10.5, color: "var(--ink-2)", background: "var(--paper-3)",
                    border: "var(--hairline)", borderRadius: 20, padding: "4px 12px" }}>{m}</span>
                ))}
              </div>
            </EcoNode>
          </EcoZone>

          {/* Cloud / team — two-way knowledge */}
          <EcoZone label="Team & cloud — knowledge, two-way" kanji="結" note="self-hosted or SaaS" accent="var(--success)"
            style={{ flex: 1, display: "flex", flexDirection: "column", gap: 14 }}>
            {/* two-way flow legend */}
            <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 7, fontSize: 11.5, color: "var(--ink-2)", background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 20, padding: "4px 12px" }}>
                <span style={{ color: "var(--accent)", fontSize: 14 }}>↑</span> you push / publish lessons up
              </span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 7, fontSize: 11.5, color: "var(--ink-2)", background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 20, padding: "4px 12px" }}>
                <span style={{ color: "var(--success)", fontSize: 14 }}>↓</span> consolidated knowledge comes back down
              </span>
            </div>
            <div style={{ display: "flex", alignItems: "stretch", gap: 14, flex: 1 }}>
              <EcoNode k="結" title="Dōjō · service + web" sub="dojo.sensei-hq.com · or your VPC" tone="var(--success)" style={{ flex: 1 }}>
                <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5, marginBottom: 6 }}>
                  The team/org hive-mind: individuals contribute → triage → approve → <b style={{ fontWeight: 600, color: "var(--ink)" }}>consolidated practice flows back</b> to everyone in the team. Every login lands here.
                </div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <span className="mono" style={{ fontSize: 10, color: "var(--success)", background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: 20, padding: "4px 12px" }}>self-host · in-house</span>
                  <span className="mono" style={{ fontSize: 10, color: "var(--ink-2)", background: "var(--paper-3)", border: "var(--hairline)", borderRadius: 20, padding: "4px 12px" }}>or managed SaaS</span>
                </div>
              </EcoNode>
              <EcoNode k="群" title="Global Collective" sub="public commons · anonymized" tone="var(--ink-2)" style={{ flex: "0 0 250px" }}>
                <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.5 }}>Opt-in public knowledge across all Sensei users — consolidated learnings flow back down, separate from any Dōjō.</div>
              </EcoNode>
            </div>
          </EcoZone>
        </div>
      </div>

      {/* legend */}
      <div style={{ display: "flex", alignItems: "center", gap: 20, marginTop: 12, paddingTop: 14, borderTop: "var(--hairline)", flexWrap: "wrap" }}>
        <span style={{ fontSize: 10.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600 }}>Boundaries</span>
        {[["机", "Your machine — code & sessions stay local", "var(--ink)"],
          ["門", "Gateway — embedded, your keys, your models", "var(--accent)"],
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
