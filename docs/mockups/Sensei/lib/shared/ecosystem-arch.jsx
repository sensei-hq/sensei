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
    <div className="border border-paper-edge flex flex-col" style={{ background: solid ? "var(--paper-3)" : "var(--paper-2)",
 borderRadius: 10, padding: "12px 16px", gap: 6, ...style }}>
      <div className="flex items-center" style={{ gap: 9 }}>
        <span className="kanji shrink-0" style={{ fontSize: 20, color: tone, lineHeight: 1 }}>{k}</span>
        <div className="min-w-0" >
          <div className="text-ink font-medium" style={{ fontSize: 13.5 }}>{title}</div>
          {sub && <div className="mono text-ink-4" style={{ fontSize: 10, marginTop: 1 }}>{sub}</div>}
        </div>
      </div>
      {children}
    </div>
  );
}

function EcoZone({ label, kanji, note, accent, children, style }) {
  return (
    <div className="relative" style={{ border: `1.5px dashed color-mix(in oklch, ${accent} 45%, var(--edge))`,
 borderRadius: 16, padding: "24px 16px 16px", background: `color-mix(in oklch, ${accent} 4%, transparent)`, ...style }}>
      <div className="absolute inline-flex items-center bg-paper" style={{ top: -11, left: 18, gap: 7, padding: "0 12px" }}>
        <span className="kanji" style={{ fontSize: 15, color: accent }}>{kanji}</span>
        <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 10.5, letterSpacing: ".14em" }}>{label}</span>
        {note && <span className="text-ink-4" style={{ fontSize: 10.5 }}>· {note}</span>}
      </div>
      {children}
    </div>
  );
}

function EcoLink({ label, sub, dir = "→" }) {
  return (
    <div className="flex flex-col items-center justify-center" style={{ gap: 4, padding: "0 4px", minWidth: 92 }}>
      <div className="uppercase text-ink-3 font-semibold text-center" style={{ fontSize: 10, letterSpacing: ".1em" }}>{label}</div>
      <div className="text-accent" style={{ fontSize: 20, lineHeight: 1 }}>{dir}</div>
      {sub && <div className="mono text-ink-4 text-center" style={{ fontSize: 9.5 }}>{sub}</div>}
    </div>
  );
}

function EcosystemArchitecture() {
  const byok = ["Claude", "GPT-4o", "Qwen 2.5", "Kimi K2", "…your keys"];
  return (
    <div className="sensei bg-paper flex flex-col" data-screen-label="Ecosystem architecture" data-om-raster
 style={{ width: 1600, minHeight: 1000, padding: 48, gap: 8, boxSizing: "border-box" }}>
      {/* title */}
      <div className="flex items-start" style={{ gap: 16, marginBottom: 8 }}>
        <span className="kanji text-accent" style={{ fontSize: 44, lineHeight: 1 }}>系</span>
        <div>
          <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".2em" }}>Sensei · system architecture</div>
          <h1 className="display font-light text-ink" style={{ fontSize: 34, letterSpacing: "-0.02em", margin: "4px 0 0" }}>The ecosystem, end to end</h1>
          <p className="text-ink-2" style={{ fontSize: 14, lineHeight: 1.6, margin: "8px 0 0", maxWidth: 940 }}>
            Local-first on the developer's machine; inference through one embedded gateway — fast by default, your keys when you want them;
            knowledge shared <b className="font-semibold text-ink" >both ways</b> with a Dōjō you host or subscribe to; a phone/pad companion for the human-in-the-loop moments.
          </p>
        </div>
      </div>

      <div className="flex items-stretch flex-1" style={{ gap: 4 }}>
        {/* ── Developer machine ── */}
        <EcoZone label="On the developer's machine" kanji="机" note="local-first" accent="var(--ink)"
          style={{ flex: "0 0 560px", display: "flex", flexDirection: "column", gap: 16 }}>
          <EcoNode k="観" title="Sensei · desktop app" sub="Tauri · observes every session" tone="var(--accent)">
            <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5, marginBottom: 4 }}>Wraps the local runtime — your code and sessions never leave unless you share.</div>
            <div className="grid" style={{ gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <EcoNode k="令" title="CLI" sub="sensei run" tone="var(--ink-3)" solid />
              <EcoNode k="具" title="MCP servers" sub="tools · instruments" tone="var(--ink-3)" solid />
              <EcoNode k="常" title="daemon" sub="always-on watcher" tone="var(--ink-3)" solid />
              <EcoNode k="蔵" title="Postgres DB" sub="local store" tone="var(--ink-3)" solid />
            </div>
            {/* Gateway — a referenced library inside the app, embeds Ollama */}
            <EcoNode k="門" title="Sensei Gateway" sub="referenced library · in-app" tone="var(--accent)"
              style={{ marginTop: 2, border: "1px solid color-mix(in oklch, var(--accent) 40%, var(--edge))" }}>
              <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5 }}>
                One inference entry point, linked into the desktop app. Picks the model per task and carries your keys.
              </div>
              <div className="bg-paper border border-paper-edge" style={{ borderRadius: 8, padding: "12px 12px", marginTop: 4 }}>
                <div className="flex items-center" style={{ gap: 8 }}>
                  <span className="kanji text-accent" style={{ fontSize: 16 }}>火</span>
                  <span className="text-ink font-medium flex-1" style={{ fontSize: 12.5 }}>Embedded Ollama</span>
                  <span className="mono text-accent bg-accent-soft" style={{ fontSize: 9.5, border: "1px solid var(--accent-edge)", borderRadius: 20, padding: "4px 8px" }}>default · Gemma 4</span>
                </div>
                <div className="text-ink-4" style={{ fontSize: 10.5, marginTop: 5 }}>bundled · fast local inference, no setup</div>
              </div>
            </EcoNode>
          </EcoNode>
          <EcoNode k="携" title="Relay · mobile / pad" sub="the away-from-keyboard companion" tone="var(--success)">
            <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5 }}>
              Pairs with the daemon over an <b className="font-semibold text-ink" >encrypted channel</b> — approve gated actions, answer decisions, watch progress while away.
            </div>
            <div className="flex items-center text-ink-3" style={{ gap: 6, marginTop: 4, fontSize: 10.5 }}>
              <span className="text-ink-4" style={{ fontSize: 14 }}>↑↓</span> encrypted pairing · to the daemon
            </div>
          </EcoNode>
        </EcoZone>

        {/* links from machine */}
        <div className="flex flex-col justify-around" >
          <EcoLink label="inference" sub="gateway → models" dir="⇄" />
          <EcoLink label="knowledge" sub="push ↑ · consolidated ↓" dir="⇄" />
        </div>

        {/* ── right stack: inference + cloud ── */}
        <div className="flex-1 flex flex-col min-w-0" style={{ gap: 16 }}>
          {/* Inference targets the gateway can also use */}
          <EcoZone label="Other inference the gateway can use" kanji="択" note="optional · BYOK" accent="var(--accent)"
            style={{ display: "flex", alignItems: "stretch", gap: 12 }}>
            <EcoNode k="火" title="Ollama service" sub="local or external · your GPUs" tone="var(--ink-2)" style={{ flex: 1 }}>
              <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5 }}>Point the gateway at a bigger Ollama host when the embedded one isn't enough.</div>
            </EcoNode>
            <EcoNode k="雲" title="BYOK cloud models" sub="bring your own keys" tone="var(--ink-2)" style={{ flex: 1 }}>
              <div className="flex flex-wrap" style={{ gap: 6, marginTop: 2 }}>
                {byok.map(m => (
                  <span key={m} className="mono text-ink-2 bg-paper-3 border border-paper-edge" style={{ fontSize: 10.5, borderRadius: 20, padding: "4px 12px" }}>{m}</span>
                ))}
              </div>
            </EcoNode>
          </EcoZone>

          {/* Cloud / team — two-way knowledge */}
          <EcoZone label="Team & cloud — knowledge, two-way" kanji="結" note="self-hosted or SaaS" accent="var(--success)"
            style={{ flex: 1, display: "flex", flexDirection: "column", gap: 14 }}>
            {/* two-way flow legend */}
            <div className="flex flex-wrap" style={{ gap: 10 }}>
              <span className="inline-flex items-center text-ink-2 bg-paper-2 border border-paper-edge" style={{ gap: 7, fontSize: 11.5, borderRadius: 20, padding: "4px 12px" }}>
                <span className="text-accent" style={{ fontSize: 14 }}>↑</span> you push / publish lessons up
              </span>
              <span className="inline-flex items-center text-ink-2 bg-paper-2 border border-paper-edge" style={{ gap: 7, fontSize: 11.5, borderRadius: 20, padding: "4px 12px" }}>
                <span className="text-success" style={{ fontSize: 14 }}>↓</span> consolidated knowledge comes back down
              </span>
            </div>
            <div className="flex items-stretch flex-1" style={{ gap: 14 }}>
              <EcoNode k="結" title="Dōjō · service + web" sub="dojo.sensei-hq.com · or your VPC" tone="var(--success)" style={{ flex: 1 }}>
                <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5, marginBottom: 6 }}>
                  The team/org hive-mind: individuals contribute → triage → approve → <b className="font-semibold text-ink" >consolidated practice flows back</b> to everyone in the team. Every login lands here.
                </div>
                <div className="flex flex-wrap" style={{ gap: 6 }}>
                  <span className="mono text-success bg-success-soft" style={{ fontSize: 10, border: "1px solid var(--success-edge)", borderRadius: 20, padding: "4px 12px" }}>self-host · in-house</span>
                  <span className="mono text-ink-2 bg-paper-3 border border-paper-edge" style={{ fontSize: 10, borderRadius: 20, padding: "4px 12px" }}>or managed SaaS</span>
                </div>
              </EcoNode>
              <EcoNode k="群" title="Global Collective" sub="public commons · anonymized" tone="var(--ink-2)" style={{ flex: "0 0 250px" }}>
                <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5 }}>Opt-in public knowledge across all Sensei users — consolidated learnings flow back down, separate from any Dōjō.</div>
              </EcoNode>
            </div>
          </EcoZone>
        </div>
      </div>

      {/* legend */}
      <div className="flex items-center border-t flex-wrap" style={{ gap: 20, marginTop: 12, paddingTop: 14 }}>
        <span className="uppercase text-ink-4 font-semibold" style={{ fontSize: 10.5, letterSpacing: ".14em" }}>Boundaries</span>
        {[["机", "Your machine — code & sessions stay local", "var(--ink)"],
          ["門", "Gateway — embedded, your keys, your models", "var(--accent)"],
          ["結", "Dōjō — hosted by you or as SaaS", "var(--success)"],
          ["群", "Collective — public, opt-in", "var(--ink-2)"]].map(([k, t, c]) => (
          <span className="inline-flex items-center text-ink-2" key={t} style={{ gap: 7, fontSize: 12 }}>
            <span className="kanji" style={{ fontSize: 14, color: c }}>{k}</span>{t}
          </span>
        ))}
      </div>
    </div>
  );
}

window.EcosystemArchitecture = EcosystemArchitecture;
