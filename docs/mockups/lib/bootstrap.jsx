// Sensei — Bootstrap screen.
// Runs on every startup. Checks prerequisites are installed + running
// before the wizard or observatory load.
//
// Gates (sequential):
//   1 Homebrew          — base. If missing: link to brew.sh.
//   2 Postgres          — brew install postgresql
//   3 Ollama            — brew install ollama
//   4 Sensei components — MCP · CLI · daemon (brew install from sensei brewfile)
//   5 Database          — sensei db:create; fallback: psql command + DATABASE_URL
//   6 Daemon            — starts once DB is reachable
//
// Once all green, caller navigates to Empty State (first run) or Observatory.

const { useState: bsUseS, useEffect: bsUseE, useMemo: bsUseM } = React;

// ─── Gate definitions ───────────────────────────────────────
const BOOT_GATES = [
  {
    id: "homebrew",   n: "一", name: "Homebrew",
    detail: "package manager",
    check: "which brew",
    remedy: "install",
  },
  {
    id: "postgres",   n: "二", name: "PostgreSQL",
    detail: "storage · @16",
    check: "brew list postgresql@16",
    remedy: "brew",
  },
  {
    id: "ollama",     n: "三", name: "Ollama",
    detail: "local models for embeddings",
    check: "brew list ollama",
    remedy: "brew",
  },
  {
    id: "sensei",     n: "四", name: "Sensei components",
    detail: "MCP · CLI · daemon",
    check: "sensei --version",
    remedy: "brew",
    sub: [
      { id: "cli",    name: "sensei-cli",    check: "sensei --version" },
      { id: "mcp",    name: "MCP bridge",    check: "sensei mcp --check" },
      { id: "daemon", name: "sensei-daemon", check: "sensei daemon --check" },
    ],
  },
  {
    id: "database",   n: "五", name: "Database",
    detail: "sensei schema · pgvector",
    check: "sensei db:create",
    remedy: "db",
  },
  {
    id: "daemon",     n: "六", name: "Daemon",
    detail: "background observer",
    check: "sensei daemon:start",
    remedy: "daemon",
  },
];

// ─── Preset scenarios for the preview/tweak ─────────────────
// Maps gate id → state. Earlier gates failing stalls later ones at "pending".
const BOOT_PRESETS = {
  "all-checking": { phase: "checking", failAt: null },
  "missing-homebrew": {
    statuses: { homebrew: "missing", postgres: "pending", ollama: "pending",
                sensei: "pending", database: "pending", daemon: "pending" }
  },
  "missing-prereqs": {
    statuses: { homebrew: "ready", postgres: "missing", ollama: "missing",
                sensei: "missing", database: "pending", daemon: "pending" }
  },
  "missing-db": {
    statuses: { homebrew: "ready", postgres: "ready", ollama: "ready",
                sensei: "ready", database: "error",   daemon: "pending" }
  },
  "daemon-starting": {
    statuses: { homebrew: "ready", postgres: "ready", ollama: "ready",
                sensei: "ready", database: "ready",   daemon: "starting" }
  },
  "all-green": {
    statuses: { homebrew: "ready", postgres: "ready", ollama: "ready",
                sensei: "ready", database: "ready",   daemon: "ready" }
  },
};

// ─── Bootstrap shell ────────────────────────────────────────
function Bootstrap({ scenario = "missing-prereqs", onReady, onSkip }) {
  const preset = BOOT_PRESETS[scenario] || BOOT_PRESETS["missing-prereqs"];
  const initial = preset.statuses ||
    BOOT_GATES.reduce((a, g, i) => (a[g.id] = i === 0 ? "checking" : "pending", a), {});

  const [statuses, setStatuses] = bsUseS(initial);
  const [dbUrl, setDbUrl] = bsUseS("postgresql://localhost:5432/sensei");
  const [dbUrlFocused, setDbUrlFocused] = bsUseS(false);

  // Reset when the preview scenario changes
  bsUseE(() => {
    setStatuses(preset.statuses ||
      BOOT_GATES.reduce((a, g, i) => (a[g.id] = i === 0 ? "checking" : "pending", a), {}));
  }, [scenario]);

  // All ready → auto-advance after a beat
  const allReady = BOOT_GATES.every(g => statuses[g.id] === "ready");
  bsUseE(() => {
    if (allReady && onReady) {
      const t = setTimeout(() => onReady(), 900);
      return () => clearTimeout(t);
    }
  }, [allReady]);

  // First blocked gate — shows its remedy panel expanded
  const firstBlockedIdx = BOOT_GATES.findIndex(g => {
    const s = statuses[g.id];
    return s === "missing" || s === "error";
  });

  // Progress for the thin rail on the side
  const readyCount = BOOT_GATES.filter(g => statuses[g.id] === "ready").length;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%',
                   background: 'var(--paper)', color: 'var(--ink)' }}>
      <TauriChrome title="Sensei  先生  ·  bootstrap"/>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr', minHeight: 0,
                     overflow: 'auto' }}>
        <div style={{ maxWidth: 760, width: '100%', margin: '0 auto',
                       padding: '48px 32px 48px', display: 'flex',
                       flexDirection: 'column', gap: 32 }}>

          {/* ── Header ──────────────────────────────── */}
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
              <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>支</span>
              <span style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                              color: 'var(--ink-3)' }}>
                bootstrap · checking the foundation
              </span>
            </div>
            <h1 className="display" style={{ fontSize: 40, fontWeight: 300, lineHeight: 1.12,
                          margin: '0 0 12px', letterSpacing: '-0.015em' }}>
              {allReady
                ? <>The foundation <span style={{ color: 'var(--success)' }}>holds.</span></>
                : firstBlockedIdx >= 0
                  ? <>A few pieces are <span style={{ color: 'var(--accent)' }}>missing.</span></>
                  : <>Checking the foundation…</>}
            </h1>
            <p style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7, margin: 0,
                         maxWidth: 540 }}>
              {allReady
                ? "Homebrew, Postgres, Ollama, sensei components, database, and the daemon are all present. Opening the observatory."
                : firstBlockedIdx >= 0
                  ? "Sensei needs these to run locally. Install the missing pieces below — the rest will check themselves once the foundation is in place."
                  : "Verifying Homebrew, Postgres, Ollama, and the sensei components. This takes a few seconds on a cold start."}
            </p>
          </div>

          {/* ── Progress rail ──────────────────────── */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                           color: 'var(--ink-4)', fontFeatureSettings: '"tnum"' }}>
              {String(readyCount).padStart(2, "0")} <span style={{ color: 'var(--ink-4)' }}>/ {String(BOOT_GATES.length).padStart(2, "0")} ready</span>
            </div>
            <div style={{ flex: 1, display: 'flex', gap: 4 }}>
              {BOOT_GATES.map((g, i) => {
                const s = statuses[g.id];
                const color = s === "ready" ? 'var(--success)'
                            : s === "checking" || s === "starting" ? 'var(--ink-2)'
                            : s === "missing" || s === "error" ? 'var(--accent)'
                            : 'var(--edge)';
                return <span key={g.id} style={{
                  flex: 1, height: 2, borderRadius: 1, background: color,
                  transition: 'background .3s', opacity: s === "pending" ? 0.5 : 1
                }}/>;
              })}
            </div>
          </div>

          {/* ── Gate list ─────────────────────────── */}
          <div style={{ display: 'flex', flexDirection: 'column',
                         borderTop: 'var(--hairline)' }}>
            {BOOT_GATES.map((gate, i) => {
              const status = statuses[gate.id];
              const showRemedy = i === firstBlockedIdx;
              return (
                <BootGate key={gate.id}
                          gate={gate}
                          status={status}
                          isFirstBlocked={showRemedy}
                          dbUrl={dbUrl}
                          setDbUrl={setDbUrl}
                          onRetry={() => {
                            // In the prototype just flip the affected gate → checking → ready
                            setStatuses(s => ({ ...s, [gate.id]: "checking" }));
                            setTimeout(() => {
                              setStatuses(s => {
                                const next = { ...s, [gate.id]: "ready" };
                                // unblock next gate as checking
                                const ni = BOOT_GATES.findIndex(g => g.id === gate.id) + 1;
                                if (ni < BOOT_GATES.length && next[BOOT_GATES[ni].id] !== "ready") {
                                  next[BOOT_GATES[ni].id] = "checking";
                                  setTimeout(() => {
                                    setStatuses(s2 => {
                                      const n2 = { ...s2 };
                                      // cascade: mark any still-pending as ready too
                                      BOOT_GATES.slice(ni).forEach(g => { n2[g.id] = "ready"; });
                                      return n2;
                                    });
                                  }, 900);
                                }
                                return next;
                              });
                            }, 1100);
                          }}/>
              );
            })}
          </div>

          {/* ── Footer ────────────────────────────── */}
          <div style={{ display: 'flex', justifyContent: 'space-between',
                         alignItems: 'center', gap: 16,
                         paddingTop: 24, borderTop: 'var(--hairline)' }}>
            <div style={{ fontSize: 11, color: 'var(--ink-4)', lineHeight: 1.6 }}>
              Bootstrap runs on every launch. Once a gate is green it'll stay that way — the next startup is quick.
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              {onSkip && (
                <button onClick={onSkip}
                        style={{ fontSize: 13, color: 'var(--ink-3)',
                                 padding: '8px 12px', border: 'none', background: 'transparent' }}>
                  Quit
                </button>
              )}
              {allReady && (
                <button onClick={onReady}
                        style={{ fontSize: 13, background: 'var(--ink)', color: 'var(--paper)',
                                 padding: '8px 24px', borderRadius: 6, letterSpacing: 0.2,
                                 border: 'none', cursor: 'pointer' }}>
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

// ─── One gate row ───────────────────────────────────────────
function BootGate({ gate, status, isFirstBlocked, dbUrl, setDbUrl, onRetry }) {
  const isBusy = status === "checking" || status === "starting";
  const isBlocked = status === "missing" || status === "error";
  const isReady = status === "ready";
  const isPending = status === "pending";

  return (
    <div style={{ borderBottom: 'var(--hairline)', padding: '16px 0',
                   opacity: isPending ? 0.42 : 1,
                   transition: 'opacity .3s' }}>
      {/* Main row */}
      <div style={{ display: 'grid', gridTemplateColumns: '32px 1fr auto', gap: 16,
                     alignItems: 'center' }}>
        <div className="kanji" style={{
          fontSize: 22,
          color: isReady ? 'var(--success)'
               : isBlocked ? 'var(--accent)'
               : isBusy ? 'var(--ink-2)'
               : 'var(--ink-4)',
          textAlign: 'center'
        }}>{gate.n}</div>

        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
            <div className="display" style={{ fontSize: 17, fontWeight: 400 }}>{gate.name}</div>
            <div style={{ fontSize: 13, color: 'var(--ink-4)' }}>· {gate.detail}</div>
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-4)', fontFamily: 'var(--font-mono)',
                         marginTop: 4 }}>
            {gate.check}
          </div>
        </div>

        <StatusPill status={status}/>
      </div>

      {/* Sub-check breakdown — only for sensei-components while busy or blocked */}
      {gate.sub && (isBusy || isBlocked || isReady) && (
        <div style={{ marginTop: 12, marginLeft: 48, display: 'flex', flexDirection: 'column',
                       gap: 4, paddingLeft: 12, borderLeft: '1px dashed var(--edge)' }}>
          {gate.sub.map((s, i) => {
            const sStatus = isReady ? "ready" : isBusy ? (i === 0 ? "checking" : "pending") : "missing";
            return (
              <div key={s.id} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <StatusDot status={sStatus}/>
                <span style={{ fontSize: 13, color: 'var(--ink-2)' }}>{s.name}</span>
                <span style={{ fontSize: 11, fontFamily: 'var(--font-mono)', color: 'var(--ink-4)' }}>
                  {s.check}
                </span>
              </div>
            );
          })}
        </div>
      )}

      {/* Remedy — only the first blocked gate expands */}
      {isFirstBlocked && isBlocked && (
        <GateRemedy gate={gate} dbUrl={dbUrl} setDbUrl={setDbUrl} onRetry={onRetry}/>
      )}
    </div>
  );
}

// ─── Status pill ────────────────────────────────────────────
function StatusPill({ status }) {
  const map = {
    ready:    { label: "ready",     color: "var(--success)", bg: "rgba(122,158,98,.10)" },
    checking: { label: "checking",  color: "var(--ink-2)", bg: "var(--paper-2)" },
    starting: { label: "starting",  color: "var(--ink-2)", bg: "var(--paper-2)" },
    missing:  { label: "missing",   color: "var(--accent)",    bg: "rgba(192,71,45,.08)" },
    error:    { label: "blocked",   color: "var(--accent)",    bg: "rgba(192,71,45,.08)" },
    pending:  { label: "waiting",   color: "var(--ink-4)", bg: "transparent" },
  };
  const m = map[status] || map.pending;
  const isBusy = status === "checking" || status === "starting";
  return (
    <div style={{ display: 'inline-flex', alignItems: 'center', gap: 4,
                   padding: '4px 8px', borderRadius: 4, background: m.bg,
                   fontSize: 11, letterSpacing: '0.08em', textTransform: 'uppercase',
                   color: m.color, fontFeatureSettings: '"tnum"' }}>
      {isBusy && <Spinner/>}
      {status === "ready"   && <span style={{ fontSize: 11 }}>✓</span>}
      {(status === "missing" || status === "error") && <span style={{ fontSize: 13 }}>·</span>}
      {m.label}
    </div>
  );
}

function StatusDot({ status }) {
  const color = status === "ready" ? 'var(--success)'
              : status === "checking" ? 'var(--ink-2)'
              : status === "missing" ? 'var(--accent)'
              : 'var(--ink-4)';
  return <span style={{ width: 6, height: 6, borderRadius: 3, background: color,
                         opacity: status === "pending" ? 0.4 : 1,
                         display: 'inline-block' }}/>;
}

// Spinner — two-dot rotation
function Spinner() {
  return (
    <span style={{ display: 'inline-block', width: 10, height: 10, position: 'relative' }}>
      <span style={{
        position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
        border: '1.5px solid currentColor', borderTopColor: 'transparent',
        borderRadius: '50%', animation: 'bs-spin 0.9s linear infinite'
      }}/>
      <style>{`@keyframes bs-spin { to { transform: rotate(360deg); } }`}</style>
    </span>
  );
}

// ─── Remedy panels ──────────────────────────────────────────
function GateRemedy({ gate, dbUrl, setDbUrl, onRetry }) {
  const copy = (s) => navigator.clipboard && navigator.clipboard.writeText(s);

  // Homebrew — send to brew.sh
  if (gate.remedy === "install") {
    return (
      <RemedyShell title="Install Homebrew" intro="Homebrew is the base that installs everything else. Run the command from the official installer, then return here and retry.">
        <CommandBlock cmd='/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'/>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginTop: 12 }}>
          <a href="https://brew.sh" target="_blank" rel="noreferrer"
             style={{ fontSize: 13, color: 'var(--ink)', textDecoration: 'none',
                       padding: '8px 12px', border: 'var(--hairline)', borderRadius: 5,
                       display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            Open brew.sh <span style={{ color: 'var(--ink-3)' }}>↗</span>
          </a>
          <button onClick={onRetry}
                  style={{ fontSize: 13, background: 'var(--ink)', color: 'var(--paper)',
                           padding: '8px 16px', borderRadius: 5, border: 'none', cursor: 'pointer' }}>
            I've installed it — retry
          </button>
        </div>
      </RemedyShell>
    );
  }

  // Brew install — sensei brewfile for prereqs + components
  if (gate.remedy === "brew") {
    return (
      <RemedyShell
        title={`Install ${gate.name.toLowerCase()} via Homebrew`}
        intro={
          gate.id === "sensei"
            ? "sensei-cli, the MCP bridge, and the daemon install together from the sensei brewfile."
            : "One line. Homebrew will handle dependencies."
        }>
        <CommandBlock cmd={
          gate.id === "sensei"
            ? "brew bundle --file=$(curl -fsSL https://sensei.dev/Brewfile)"
            : gate.id === "postgres"
              ? "brew install postgresql@16 && brew services start postgresql@16"
              : "brew install ollama && brew services start ollama"
        }/>
        <div style={{ fontSize: 11, color: 'var(--ink-4)', lineHeight: 1.6, marginTop: 8 }}>
          Or install everything sensei needs in one pass:
        </div>
        <CommandBlock cmd="brew bundle --file=$(curl -fsSL https://sensei.dev/Brewfile)" muted/>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginTop: 12 }}>
          <a href="https://github.com/sensei-dev/sensei" target="_blank" rel="noreferrer"
             style={{ fontSize: 13, color: 'var(--ink)', textDecoration: 'none',
                       padding: '8px 12px', border: 'var(--hairline)', borderRadius: 5,
                       display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            View Brewfile on GitHub <span style={{ color: 'var(--ink-3)' }}>↗</span>
          </a>
          <button onClick={onRetry}
                  style={{ fontSize: 13, background: 'var(--ink)', color: 'var(--paper)',
                           padding: '8px 16px', borderRadius: 5, border: 'none', cursor: 'pointer' }}>
            Retry check
          </button>
        </div>
      </RemedyShell>
    );
  }

  // Database — couldn't create; show manual command + DATABASE_URL input
  if (gate.remedy === "db") {
    return (
      <RemedyShell
        title="Could not create the sensei database"
        intro="Postgres is running but sensei couldn't create its database automatically. Either create one manually and paste its URL, or let sensei retry.">
        <div style={{ fontSize: 11, letterSpacing: '0.1em', textTransform: 'uppercase',
                       color: 'var(--ink-4)', marginBottom: 4 }}>Manual create</div>
        <CommandBlock cmd="createdb sensei && psql sensei -c 'CREATE EXTENSION IF NOT EXISTS vector;'"/>

        <div style={{ fontSize: 11, letterSpacing: '0.1em', textTransform: 'uppercase',
                       color: 'var(--ink-4)', marginTop: 16, marginBottom: 4 }}>Database URL</div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <input value={dbUrl} onChange={e => setDbUrl(e.target.value)}
                 style={{ flex: 1, fontSize: 13, fontFamily: 'var(--font-mono)',
                          padding: '8px 8px', borderRadius: 5,
                          border: 'var(--hairline)', background: 'var(--paper)',
                          color: 'var(--ink)' }}/>
          <button onClick={onRetry}
                  style={{ fontSize: 13, background: 'var(--ink)', color: 'var(--paper)',
                           padding: '8px 16px', borderRadius: 5, border: 'none', cursor: 'pointer',
                           whiteSpace: 'nowrap' }}>
            Connect
          </button>
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-4)', lineHeight: 1.6, marginTop: 8 }}>
          Sensei stores this in <span style={{ fontFamily: 'var(--font-mono)' }}>~/.sensei/config.toml</span>. You can change it later in Settings → Database.
        </div>
      </RemedyShell>
    );
  }

  // Daemon — show logs + retry
  if (gate.remedy === "daemon") {
    return (
      <RemedyShell title="Daemon failed to start"
        intro="The database is reachable but the daemon did not come up. Here are the last lines of its log.">
        <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 5, padding: 12,
                       fontFamily: 'var(--font-mono)', fontSize: 11,
                       color: 'var(--ink-2)', lineHeight: 1.7 }}>
          <div><span style={{ color: 'var(--ink-4)' }}>10:42:18</span> daemon · starting</div>
          <div><span style={{ color: 'var(--ink-4)' }}>10:42:18</span> daemon · loading config ~/.sensei/config.toml</div>
          <div><span style={{ color: 'var(--ink-4)' }}>10:42:19</span> daemon · connecting to postgres</div>
          <div><span style={{ color: 'var(--accent)' }}>10:42:19 ERR</span> daemon · port 7714 already in use</div>
        </div>
        <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
          <button onClick={onRetry}
                  style={{ fontSize: 13, background: 'var(--ink)', color: 'var(--paper)',
                           padding: '8px 16px', borderRadius: 5, border: 'none', cursor: 'pointer' }}>
            Retry
          </button>
          <button style={{ fontSize: 13, color: 'var(--ink-2)',
                           padding: '8px 12px', border: 'var(--hairline)', borderRadius: 5,
                           background: 'transparent' }}>
            Change port
          </button>
        </div>
      </RemedyShell>
    );
  }

  return null;
}

function RemedyShell({ title, intro, children }) {
  return (
    <div style={{ marginTop: 16, marginLeft: 48, padding: '16px 16px',
                   background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 6 }}>
      <div className="display" style={{ fontSize: 15, marginBottom: 4 }}>{title}</div>
      <div style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.65, marginBottom: 12,
                     maxWidth: 580 }}>
        {intro}
      </div>
      {children}
    </div>
  );
}

function CommandBlock({ cmd, muted }) {
  const [copied, setCopied] = bsUseS(false);
  const doCopy = () => {
    if (navigator.clipboard) navigator.clipboard.writeText(cmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 1100);
  };
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                   background: muted ? 'transparent' : 'var(--paper)',
                   border: muted ? '1px dashed var(--edge)' : 'var(--hairline)',
                   borderRadius: 5, padding: '8px 12px',
                   fontFamily: 'var(--font-mono)', fontSize: 13,
                   color: muted ? 'var(--ink-3)' : 'var(--ink)' }}>
      <span style={{ color: 'var(--ink-4)', userSelect: 'none' }}>$</span>
      <span style={{ flex: 1, overflow: 'auto', whiteSpace: 'nowrap' }}>{cmd}</span>
      <button onClick={doCopy}
              style={{ fontSize: 11, letterSpacing: '0.1em', textTransform: 'uppercase',
                       color: copied ? 'var(--success)' : 'var(--ink-3)',
                       border: 'none', background: 'transparent', cursor: 'pointer',
                       padding: '4px 4px' }}>
        {copied ? "copied" : "copy"}
      </button>
    </div>
  );
}

// ─── Demo wrapper — cycles through scenarios for the artboard ─
function BootstrapDemo() {
  const [scenario, setScenario] = bsUseS("missing-prereqs");
  return (
    <div style={{ height: '100%', position: 'relative' }}>
      <Bootstrap scenario={scenario}
                 onReady={() => {}}
                 onSkip={() => {}}/>
      {/* Scenario picker — floating, demo-only */}
      <div style={{ position: 'absolute', top: 52, right: 16, zIndex: 5,
                     background: 'var(--paper)', border: 'var(--hairline)',
                     borderRadius: 6, padding: 8, width: 200,
                     boxShadow: '0 4px 12px rgba(0,0,0,.06)' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                       color: 'var(--ink-4)', marginBottom: 8 }}>demo · scenario</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {Object.keys(BOOT_PRESETS).map(k => (
            <button key={k} onClick={() => setScenario(k)}
                    style={{ textAlign: 'left', fontSize: 11,
                             padding: '4px 8px', borderRadius: 4, border: 'none',
                             background: scenario === k ? 'var(--paper-2)' : 'transparent',
                             color: scenario === k ? 'var(--ink)' : 'var(--ink-3)',
                             cursor: 'pointer' }}>
              {k.replace(/-/g, ' ')}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { Bootstrap, BootstrapDemo, BOOT_GATES, BOOT_PRESETS });
