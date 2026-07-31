// ─── Bootstrap (alt · simplified) ───────────────────────────
// A simpler take on the startup health-check. Same job — verify
// Homebrew/winget + Postgres + Ollama + sensei components + DB +
// daemon — but expressed as ONE big action instead of six gates.
//
// Mental model:
//   "One door, one key."
//   The key is the platform package manager (Homebrew on Mac/Linux,
//   winget on Windows). If the key is present, sensei can fix
//   everything else itself by running a Brewfile/winget manifest
//   from sensei-hq/homebrew-tap. If the key is missing — or the
//   automated fix hits something it can't resolve — we hand the
//   user a copy-pasteable script and a "re-check" button.
//
// States the screen can be in:
//   • detecting   — initial probe running
//   • all-green   — every dep present and the daemon is up
//   • auto-fixing — brew bundle / winget import is running now
//   • manual      — auto-fix failed or the package manager itself
//                   is missing; show the user a script to run
//
// The visual hierarchy collapses to a single hero card. The detail
// rows live below it as small mono lines, so the user can glance
// and not have to read.

const { useState: bsSimpleUseS, useEffect: bsSimpleUseE } = React;

// ─── Items that the package-manager step resolves ───────────
// Each item maps to a real check sensei runs after the bundle.
const BS_ITEMS = [
  { id: "postgres",  label: "PostgreSQL @16",      group: "system" },
  { id: "ollama",    label: "Ollama",              group: "system" },
  { id: "sensei",    label: "Sensei components",   group: "sensei", note: "cli · mcp · daemon" },
  { id: "database",  label: "Database & schema",   group: "sensei", note: "pgvector · sensei tables" },
  { id: "daemon",    label: "Background daemon",   group: "sensei" },
];

// ─── Per-platform install command + script preview ──────────
const BS_PLATFORMS = {
  mac: {
    label: "macOS",
    pmName: "Homebrew",
    pmCheck: "which brew",
    bundleCmd: "brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile",
    installPm: '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
    fullScript: [
      "# Install Homebrew (only if missing)",
      'if ! command -v brew >/dev/null 2>&1; then',
      '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
      'fi',
      "",
      "# Install everything sensei needs in one shot",
      "brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile",
      "",
      "# Initialise the database & start the daemon",
      "sensei db:create && sensei daemon:start",
    ].join("\n"),
  },
  linux: {
    label: "Linux",
    pmName: "Homebrew",
    pmCheck: "which brew",
    bundleCmd: "brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile",
    installPm: '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
    fullScript: [
      "# Install Homebrew (only if missing)",
      'if ! command -v brew >/dev/null 2>&1; then',
      '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
      '  eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"',
      'fi',
      "",
      "brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile",
      "sensei db:create && sensei daemon:start",
    ].join("\n"),
  },
  windows: {
    label: "Windows",
    pmName: "winget",
    pmCheck: "winget --version",
    bundleCmd: "winget import --import-file sensei.winget.json --accept-package-agreements",
    installPm: "# winget ships with Windows 11 — install \"App Installer\" from the Microsoft Store on Windows 10",
    fullScript: [
      "# Verify winget (ships with Windows 11; install App Installer on Windows 10)",
      "winget --version",
      "",
      "# Pull sensei's manifest and run a single import",
      "iwr -useb https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/sensei.winget.json -OutFile sensei.winget.json",
      "winget import --import-file sensei.winget.json --accept-package-agreements --accept-source-agreements",
      "",
      "# Initialise the database & start the daemon",
      "sensei db:create",
      "sensei daemon:start",
    ].join("\n"),
  },
};

// ─── Preset scenarios for the artboards ─────────────────────
const BS_SIMPLE_PRESETS = {
  "auto-fixing-mac":     { platform: "mac",     state: "auto-fixing" },
  "auto-fixing-linux":   { platform: "linux",   state: "auto-fixing" },
  "all-green-mac":       { platform: "mac",     state: "all-green" },
  "manual-mac-no-brew":  { platform: "mac",     state: "manual",     reason: "no-brew" },
  "manual-windows":      { platform: "windows", state: "manual",     reason: "no-winget" },
  "manual-bundle-err":   { platform: "mac",     state: "manual",     reason: "bundle-error" },
};

// ─── Root component ─────────────────────────────────────────
function BootstrapSimple({ scenario = "auto-fixing-mac", onReady, onSkip }) {
  const preset = BS_SIMPLE_PRESETS[scenario] || BS_SIMPLE_PRESETS["auto-fixing-mac"];
  const platform = BS_PLATFORMS[preset.platform];

  const [state, setState] = bsSimpleUseS(preset.state);
  const [progress, setProgress] = bsSimpleUseS(0);
  const [activeItem, setActiveItem] = bsSimpleUseS(0);
  const [copied, setCopied] = bsSimpleUseS(false);
  const [scriptOpen, setScriptOpen] = bsSimpleUseS(true);

  // Reset whenever the demo scenario changes
  bsSimpleUseE(() => {
    setState(preset.state);
    setProgress(0);
    setActiveItem(0);
    setCopied(false);
  }, [scenario]);

  // While auto-fixing, walk the items so the screen feels alive
  bsSimpleUseE(() => {
    if (state !== "auto-fixing") return;
    const total = BS_ITEMS.length;
    const stepMs = 750;
    const i = setInterval(() => {
      setActiveItem(prev => {
        const next = prev + 1;
        if (next >= total) {
          clearInterval(i);
          // Finish: fall through to all-green after a beat
          setTimeout(() => setState("all-green"), 600);
          return total;
        }
        return next;
      });
      setProgress(p => Math.min(100, p + 100 / total));
    }, stepMs);
    return () => clearInterval(i);
  }, [state]);

  // Auto-advance once green
  bsSimpleUseE(() => {
    if (state === "all-green" && onReady) {
      const t = setTimeout(() => onReady(), 1200);
      return () => clearTimeout(t);
    }
  }, [state]);

  const copyScript = () => {
    const txt = platform.fullScript;
    if (navigator.clipboard) {
      navigator.clipboard.writeText(txt).catch(() => {});
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  const recheck = () => {
    setState("auto-fixing");
    setActiveItem(0);
    setProgress(0);
  };

  const reasonCopy = {
    "no-brew":      `${platform.pmName} isn't installed yet. Run the script below — it installs ${platform.pmName} first, then everything else.`,
    "no-winget":    "winget isn't available on this system. Install \"App Installer\" from the Microsoft Store, then run the script below.",
    "bundle-error": `${platform.pmName} is here, but ${platform.bundleCmd.split(" ")[0]} hit a permission issue. The script below runs the same steps with sudo where needed.`,
  }[preset.reason] || `Sensei can't finish this on its own. Run the script below — it'll do the rest.`;

  return (
    <div className="flex flex-col h-full bg-paper text-ink" >
      <TauriChrome title="Sensei  先生  ·  bootstrap"/>

      <div className="py-12 px-8 flex-1 overflow-auto flex justify-center items-start" >
        <div className="w-full" style={{ maxWidth: 640 }}>

          {/* ── Header ──────────────────────────────── */}
          <div className="mb-8" >
            <div className="gap-2 mb-3 flex items-center" >
              <span className="kanji text-accent" style={{ fontSize: 22 }}>支</span>
              <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
                Bootstrap · {platform.label}
              </span>
            </div>
            <h1 className="display mt-0 mb-3 font-light" style={{
 fontSize: 40, lineHeight: 1.15, letterSpacing: '-0.015em'
 }}>
              {state === "all-green"
                ? <>The foundation <span className="text-success" >holds.</span></>
              : state === "auto-fixing"
                ? <>Setting up your <span className="text-accent" >foundation.</span></>
              : <>One last <span className="text-accent" >step.</span></>}
            </h1>
            <p style={{
 fontSize: 13, lineHeight: 1.65,
 maxWidth: 540
 }} className="m-0 text-ink-3" >
              {state === "all-green"
                ? "Everything sensei needs is here. Opening the observatory."
              : state === "auto-fixing"
                ? <>Running <span className="mono text-ink-2" >{platform.bundleCmd.split(" ")[0]}</span> with the manifest from <span className="mono text-ink-2" >sensei-hq/homebrew-tap</span>. No input needed.</>
              : reasonCopy}
            </p>
          </div>

          {/* ── Hero card · the single source of truth ──────────── */}
          <BSHeroCard state={state} platform={platform}
                      activeItem={activeItem} progress={progress}
                      onContinue={onReady}/>

          {/* ── Manual fallback · script block ────────────────────
              Only shown in the manual state. The script is the same
              one sensei would have run; the user just runs it
              themselves and clicks "Re-check". */}
          {state === "manual" && (
            <BSScriptCard platform={platform} reason={preset.reason}
                          scriptOpen={scriptOpen} setScriptOpen={setScriptOpen}
                          onCopy={copyScript} copied={copied}
                          onRecheck={recheck}/>
          )}

          {/* ── Item ledger · what's resolved (collapsed by default) */}
          <BSItemLedger state={state} activeItem={activeItem}/>

          {/* ── Footer ──────────────────────────────── */}
          <div className="gap-4 mt-8 pt-6 flex justify-between items-center border-t" >
            <div className="text-ink-4" style={{ fontSize: 11, lineHeight: 1.6 }}>
              Bootstrap runs once. The next launch will be quick.
            </div>
            <div className="gap-2 flex" >
              {onSkip && (
                <button onClick={onSkip}
 style={{
 fontSize: 13 }} className="py-2 px-3 text-ink-3 border-0 bg-transparent cursor-pointer" >
                  Quit
                </button>
              )}
              {state === "all-green" && (
                <button onClick={onReady}
 style={{
 fontSize: 13, borderRadius: 6, letterSpacing: 0.2 }} className="py-2 px-6 bg-ink text-paper border-0 cursor-pointer" >
                  Continue →
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Hero card ─────────────────────────────────────────────
// One big status surface. Looks subtly different per state so the
// user never has to ask "what's happening right now?".
function BSHeroCard({ state, platform, activeItem, progress, onContinue }) {
  const isFixing = state === "auto-fixing";
  const isGreen  = state === "all-green";
  const isManual = state === "manual";

  const accent = isGreen  ? 'var(--success)'
              : isManual ? 'var(--accent)'
              : 'var(--ink-2)';

  return (
    <div style={{ borderRadius: 10 }} className="py-6 px-6 border border-paper-edge bg-paper-2 relative overflow-hidden" >
      {/* live progress bar across the top while auto-fixing */}
      {isFixing && (
        <div className="absolute" style={{ left: 0, top: 0, right: 0, height: 2,
 background: 'var(--edge)' }}>
          <div className="h-full bg-accent" style={{ width: `${progress}%`,
 transition: 'width .65s ease' }}/>
        </div>
      )}

      <div className="gap-4 flex items-center" >
        {/* Big indicator — one symbol per state */}
        <div className="bg-paper flex items-center justify-center shrink-0" style={{ width: 56, height: 56, borderRadius: 28,
 border: `1.5px solid ${accent}`,
 boxShadow: isFixing ? `0 0 0 6px ${accent}10` : 'none',
 animation: isFixing ? 'bsHaloPulse 1.6s ease-in-out infinite' : 'none' }}>
          {isGreen   && <span style={{ fontSize: 28, color: accent, lineHeight: 1 }}>✓</span>}
          {isManual  && <span className="kanji" style={{ fontSize: 22, color: accent }}>?</span>}
          {isFixing  && <span className="rounded-full" style={{
 width: 18, height: 18,
 border: `2px solid ${accent}`, borderTopColor: 'transparent',
 animation: 'bsSpin 0.9s linear infinite' }}/>}
        </div>

        <div className="flex-1 min-w-0" >
          <div className="gap-2 mb-1 flex items-baseline" >
            <span className="display font-medium" style={{ fontSize: 17 }}>
              {platform.pmName}
            </span>
            <span className="mono text-ink-4" style={{ fontSize: 11 }}>
              {platform.pmCheck}
            </span>
          </div>
          <div className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55 }}>
            {isGreen   && "Detected. All dependencies installed."}
            {isFixing  && (
              <span>
                Detected. Installing{" "}
                <span className="text-ink" >
                  {BS_ITEMS[Math.min(activeItem, BS_ITEMS.length - 1)].label}
                </span>
                <span className="mono ml-2 text-ink-4" style={{ fontSize: 11 }}>
                  ({Math.min(activeItem + 1, BS_ITEMS.length)}/{BS_ITEMS.length})
                </span>
              </span>
            )}
            {isManual  && "Couldn't finish automatically. The script below picks up from where sensei stopped."}
          </div>
        </div>

        {isGreen && (
          <button onClick={onContinue}
 style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'inherit',
 letterSpacing: 0.3 }} className="py-2 px-4 bg-success text-paper border-0 cursor-pointer shrink-0" >
            Enter
          </button>
        )}
      </div>

      <style>{`
        @keyframes bsSpin { to { transform: rotate(360deg); } }
        @keyframes bsHaloPulse {
          0%, 100% { box-shadow: 0 0 0 6px var(--edge); }
          50%      { box-shadow: 0 0 0 10px transparent; }
        }
      `}</style>
    </div>
  );
}

// ─── Script card · the manual fallback ─────────────────────
function BSScriptCard({ platform, reason, scriptOpen, setScriptOpen, onCopy, copied, onRecheck }) {
  return (
    <div style={{
 border: '1px solid var(--accent)', borderRadius: 10 }} className="mt-4 bg-paper overflow-hidden" >
      {/* Header bar */}
      <div className="py-3 px-4 gap-2 border-b flex items-center" >
        <span className="kanji text-accent" style={{ fontSize: 15 }}>手</span>
        <div className="flex-1" >
          <div className="text-ink" style={{ fontSize: 13 }}>
            Run this in your terminal
          </div>
          <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
            Same steps sensei would run automatically · {platform.label}
          </div>
        </div>
        <button onClick={() => setScriptOpen(o => !o)}
 style={{
 fontSize: 11,
 borderRadius: 4, fontFamily: 'inherit'
 }} className="py-1 px-2 bg-transparent text-ink-3 border border-paper-edge cursor-pointer" >
          {scriptOpen ? "Hide" : "Show"}
        </button>
      </div>

      {scriptOpen && (
        <>
          {/* Script body */}
          <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 13,
 lineHeight: 1.65, whiteSpace: 'pre-wrap', wordBreak: 'break-word',
 maxHeight: 220 }} className="py-4 px-4 m-0 text-ink bg-paper-3 overflow-auto" >{platform.fullScript}</pre>

          {/* Action bar */}
          <div className="py-3 px-4 gap-2 flex items-center justify-between border-t" >
            <button onClick={onCopy}
 style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'inherit',
 letterSpacing: 0.2
 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer" >
              {copied ? "Copied ✓" : "Copy script"}
            </button>
            <button onClick={onRecheck}
 style={{
 fontSize: 13, border: '1px solid var(--accent)',
 borderRadius: 5, fontFamily: 'inherit'
 }} className="py-2 px-3 bg-transparent text-accent cursor-pointer" >
              I've run it · re-check
            </button>
          </div>
        </>
      )}
    </div>
  );
}

// ─── Item ledger · what got installed ──────────────────────
// Compact list under the hero. While auto-fixing, current item is
// emphasised; resolved items get a faint check; pending ones go
// quiet. In all-green state, everything is checked off.
function BSItemLedger({ state, activeItem }) {
  return (
    <div className="mt-6" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 uppercase text-ink-4" >
        what this resolves
      </div>
      <div className="gap-0 flex flex-col" >
        {BS_ITEMS.map((it, i) => {
          let s;
          if (state === "all-green")    s = "ready";
          else if (state === "manual")  s = "blocked";
          else if (state === "auto-fixing") {
            s = i < activeItem ? "ready"
              : i === activeItem ? "running"
              : "pending";
          } else s = "pending";

          const dotColor = s === "ready"   ? 'var(--success)'
                        : s === "running" ? 'var(--accent)'
                        : s === "blocked" ? 'var(--ink-3)'
                        : 'var(--edge)';
          return (
            <div key={it.id} style={{ gridTemplateColumns: 'auto 1fr auto',
 borderBottom: '1px solid var(--edge)',
 opacity: s === "pending" ? 0.55 : 1,
 transition: 'opacity .25s' }} className="gap-3 py-2 px-0 grid items-center" >
              <span className="rounded-full" style={{ width: 8, height: 8,
 background: dotColor,
 boxShadow: s === "running" ? `0 0 0 4px ${dotColor}22` : 'none',
 animation: s === "running" ? "bsItemPulse 1.2s ease-in-out infinite" : 'none' }}/>
              <div>
                <span className="text-ink" style={{ fontSize: 13 }}>{it.label}</span>
                {it.note && (
                  <span style={{ fontSize: 11 }} className="ml-2 text-ink-4" >
                    · {it.note}
                  </span>
                )}
              </div>
              <span className="mono uppercase" style={{
 fontSize: 11,
 color: s === "ready" ? 'var(--success)'
 : s === "running" ? 'var(--accent)'
 : 'var(--ink-4)',
 letterSpacing: '0.06em' }}>
                {s}
              </span>
            </div>
          );
        })}
      </div>
      <style>{`
        @keyframes bsItemPulse { 0%,100% { opacity: 1 } 50% { opacity: 0.4 } }
      `}</style>
    </div>
  );
}

// ─── Demo wrapper · cycles through scenarios ───────────────
function BootstrapSimpleDemo() {
  const [scenario, setScenario] = bsSimpleUseS("auto-fixing-mac");
  return (
    <div className="h-full relative" >
      <BootstrapSimple scenario={scenario} onReady={() => {}} onSkip={() => {}}/>

      {/* Floating scenario picker — demo only */}
      <div style={{ top: 52, right: 16, zIndex: 5,
 borderRadius: 8,
 boxShadow: '0 4px 12px rgba(0,0,0,.06)'
 }} className="py-3 px-3 absolute bg-paper border border-paper-edge" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 uppercase text-ink-4" >demo · scenario</div>
        <div className="gap-1 flex flex-col" >
          {Object.keys(BS_SIMPLE_PRESETS).map(k => (
            <button key={k} onClick={() => setScenario(k)}
 style={{ fontSize: 11, borderRadius: 4,
 background: scenario === k ? 'var(--paper-2)' : 'transparent',
 color: scenario === k ? 'var(--ink)' : 'var(--ink-3)', fontFamily: 'inherit' }} className="py-1 px-2 text-left border-0 cursor-pointer whitespace-nowrap" >
              {k.replace(/-/g, ' ')}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  BootstrapSimple, BootstrapSimpleDemo,
  BS_ITEMS, BS_PLATFORMS, BS_SIMPLE_PRESETS,
});
