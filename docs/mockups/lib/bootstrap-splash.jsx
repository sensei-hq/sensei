// ─── Bootstrap (splash · system dialog) ─────────────────────
// The startup splash — appears every time sensei launches.
// Same six gates as the original Bootstrap, but expressed
// as a small (880×480) system dialog rather than a full app
// screen. Hides everything when green; expands into a two-
// column layout when there's work to do or report.
//
// States:
//   • probing     — initial probe is running
//   • auto-fixing — running brew bundle from sensei-hq/tap
//   • manual      — package manager itself is missing
//   • all-green   — everything ready, opening observatory
//
// TYPOGRAPHY NOTE
// ───────────────
// The design system's type scale (xs:11 / sm:13 / lg:17 / xl:22 / 2xl:28)
// is calibrated for 1280×860 product screens. The splash window is a
// real macOS system dialog at 880×480 — the same tokens applied verbatim
// render oversized. We deliberately use calibrated px (10–12.5) for the
// dialog's UI text while keeping semantic FONT-FAMILY classes (.display
// for Fraunces, .mono for JetBrains Mono, .kanji for Yu Mincho) so the
// typographic system stays coherent even though the sizes don't snap to
// the public scale.

const { useEffect: splashUseE } = React;

/* ─────────────────────────────────────────────────────────────
   STYLES — injected once so the lib file is self-contained
   ───────────────────────────────────────────────────────────── */
(function ensureSplashStyles() {
  if (document.getElementById('splash-styles')) return;
  const s = document.createElement('style');
  s.id = 'splash-styles';
  s.textContent = `
    @keyframes splashFade   { 0%,100% { opacity: 0.4; } 50% { opacity: 1; } }
    @keyframes splashInk    { 0% { opacity: 0; transform: translateY(3px); } 100% { opacity: 1; transform: translateY(0); } }
    @keyframes splashSpin   { to { transform: rotate(360deg); } }
    @keyframes splashTickle { 0%,100% { transform: scaleX(0.92); opacity: 0.6; }
                              50%     { transform: scaleX(1);    opacity: 1; } }
    @keyframes splashRevealCol {
      0%   { opacity: 0; transform: translateX(-6px); }
      100% { opacity: 1; transform: translateX(0); }
    }

    .splash-fade    { animation: splashFade 1.8s var(--ease) infinite; }
    .splash-ink     { animation: splashInk 0.6s var(--ease) both; }
    .splash-tickle  { transform-origin: left center; animation: splashTickle 2.4s var(--ease) infinite; }
    .splash-reveal  { animation: splashRevealCol 0.5s var(--ease) both; }

    .splash-window {
      width: 100%; height: 100%;
      display: flex; flex-direction: column;
      background: var(--paper);
      color: var(--ink);
      overflow: hidden;
      position: relative;
      border-radius: 10px;
    }
    .splash-chrome {
      height: 26px; display: flex; align-items: center;
      padding: 0 12px; flex-shrink: 0;
    }
    .splash-chrome .tauri-traffic { opacity: 0.45; }

    .splash-script {
      font-family: var(--font-mono);
      font-size: 11px;
      line-height: 1.65;
      color: var(--ink);
      background: var(--paper-3);
      padding: 8px 10px;
      border-radius: 4px;
      white-space: pre-wrap;
      word-break: break-word;
      max-height: 110px;
      overflow: auto;
      margin: 0;
    }

    /* Splash desktop — when the splash is shown inside a larger
       artboard it sits as a floating window on a quieter washi
       surface, like a real macOS startup dialog. */
    .splash-desktop {
      width: 100%; height: 100%;
      display: flex; align-items: center; justify-content: center;
      background:
        radial-gradient(ellipse at 50% 30%, oklch(0.945 0.012 85) 0%, oklch(0.905 0.014 85) 70%, oklch(0.875 0.016 85) 100%);
    }
    .splash-desktop > .splash-window-frame {
      width: 880px; height: 480px;
      box-shadow:
        0 0 0 1px oklch(0.22 0.012 50 / 0.06),
        0 2px 6px oklch(0.22 0.012 50 / 0.10),
        0 30px 80px oklch(0.22 0.012 50 / 0.18);
      border-radius: 10px;
    }
  `;
  document.head.appendChild(s);
})();

/* ─────────────────────────────────────────────────────────────
   MODEL
   ───────────────────────────────────────────────────────────── */

const SPLASH_GATES = [
  { id: 'homebrew', n: '一', name: 'Homebrew',          detail: 'package manager',   check: 'which brew',
    zen: 'The gardener who tends the tools.' },
  { id: 'postgres', n: '二', name: 'PostgreSQL',         detail: 'storage · @16',     check: 'brew list postgresql@16',
    zen: 'A still pond where memories settle.' },
  { id: 'ollama',   n: '三', name: 'Ollama',             detail: 'local models',      check: 'brew list ollama',
    zen: 'A mind that thinks without leaving the room.' },
  { id: 'sensei',   n: '四', name: 'Sensei components',  detail: 'cli · mcp · daemon', check: 'sensei --version',
    zen: 'Three hands of the practice — speak, listen, attend.' },
  { id: 'database', n: '五', name: 'Database',           detail: 'schema · pgvector', check: 'sensei db:create',
    zen: 'Shelves shaped to the form of each memory.' },
  { id: 'daemon',   n: '六', name: 'Daemon',             detail: 'background',        check: 'sensei daemon:start',
    zen: 'The quiet breath that keeps watch.' },
];

const SPLASH_STATE_STATUSES = {
  probing: {
    homebrew: 'checking', postgres: 'pending', ollama: 'pending',
    sensei: 'pending', database: 'pending', daemon: 'pending',
  },
  'auto-fixing': {
    homebrew: 'ready', postgres: 'ready', ollama: 'ready',
    sensei: 'installing', database: 'pending', daemon: 'pending',
  },
  manual: {
    homebrew: 'missing', postgres: 'pending', ollama: 'pending',
    sensei: 'pending', database: 'pending', daemon: 'pending',
  },
  'all-green': {
    homebrew: 'ready', postgres: 'ready', ollama: 'ready',
    sensei: 'ready', database: 'ready', daemon: 'ready',
  },
};

/* ─────────────────────────────────────────────────────────────
   PRIMITIVES
   ───────────────────────────────────────────────────────────── */

function SplashChrome() {
  return (
    <div className="splash-chrome">
      <div className="tauri-traffic"><span/><span/><span/></div>
    </div>
  );
}

function SplashWordmark({ size = 'md' }) {
  const k = size === 'lg' ? 32 : size === 'sm' ? 18 : 26;
  const w = size === 'lg' ? 26 : size === 'sm' ? 14 : 20;
  return (
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
      <span className="kanji" style={{ fontSize: k, color: 'var(--accent)', lineHeight: 1 }}>先生</span>
      <span className="display" style={{ fontSize: w, fontWeight: 400, letterSpacing: '-0.01em' }}>Sensei</span>
    </div>
  );
}

function SplashSpinner({ size = 10, color = 'currentColor' }) {
  return (
    <span style={{ display: 'inline-block', width: size, height: size, position: 'relative' }}>
      <span style={{
        position: 'absolute', inset: 0,
        border: `1.5px solid ${color}`, borderTopColor: 'transparent',
        borderRadius: '50%',
        animation: 'splashSpin 0.9s linear infinite',
      }}/>
    </span>
  );
}

function SplashStatusDisc({ status, size = 20 }) {
  const isReady     = status === 'ready';
  const isBusy      = status === 'checking' || status === 'starting' || status === 'installing';
  const isBlocked   = status === 'missing'  || status === 'error';
  const isPending   = status === 'pending';

  const color = isReady   ? 'var(--success)'
              : isBlocked ? 'var(--accent)'
              : isBusy    ? 'var(--accent)'
              : 'var(--ink-4)';

  const inner = size >= 32 ? 14 : size >= 24 ? 11 : 10;
  const stroke = size >= 32 ? 2 : 1.5;

  return (
    <div style={{
      width: size, height: size, borderRadius: '50%',
      background: 'var(--paper)',
      border: `${stroke}px solid ${color}`,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      flexShrink: 0,
      opacity: isPending ? 0.55 : 1,
      transition: 'opacity .25s, border-color .25s',
      boxShadow: isBusy ? `0 0 0 ${Math.round(size * 0.18)}px ${color}1f` : 'none',
    }}>
      {isReady && (
        <svg width={inner} height={inner} viewBox="0 0 10 10" fill="none">
          <path d="M2 5.2 L4.2 7.2 L8 3" stroke={color}
                strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      )}
      {isBusy && (
        <span style={{
          width: inner, height: inner, borderRadius: '50%',
          border: `${stroke}px solid ${color}`, borderTopColor: 'transparent',
          animation: 'splashSpin 0.9s linear infinite',
        }}/>
      )}
      {isBlocked && (
        <span className="kanji" style={{
          fontSize: Math.round(size * 0.6), color, lineHeight: 1, fontWeight: 400,
        }}>?</span>
      )}
    </div>
  );
}

function SplashStatusIndicator({ status }) {
  const labelMap = {
    checking:   'checking',
    installing: 'installing',
    starting:   'starting',
    missing:    'missing',
    error:      'blocked',
    pending:    null,
    ready:      null,
  };
  const label = labelMap[status];
  const isReady   = status === 'ready';
  const isBusy    = status === 'checking' || status === 'starting' || status === 'installing';
  const isBlocked = status === 'missing'  || status === 'error';
  const color = isReady   ? 'var(--success)'
              : isBlocked ? 'var(--accent)'
              : isBusy    ? 'var(--accent)'
              : 'var(--ink-4)';
  return (
    <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
      {label && (
        <span className="mono" style={{
          fontSize: 10, letterSpacing: '0.12em', textTransform: 'uppercase',
          color, lineHeight: 1,
        }}>{label}</span>
      )}
      <SplashStatusDisc status={status} size={20}/>
    </div>
  );
}

function splashOverallStatus(statuses) {
  const vals = Object.values(statuses);
  if (vals.some(s => s === 'missing' || s === 'error')) return 'missing';
  if (vals.some(s => s === 'installing'))               return 'installing';
  if (vals.some(s => s === 'starting'))                 return 'starting';
  if (vals.some(s => s === 'checking'))                 return 'checking';
  if (vals.every(s => s === 'ready'))                   return 'ready';
  return 'pending';
}

/* One gate row — kanji numeral · name · detail + zen line · status indicator */
function SplashGateRow({ gate, status, delay = 0 }) {
  const isReady   = status === 'ready';
  const isBusy    = status === 'checking' || status === 'starting' || status === 'installing';
  const isBlocked = status === 'missing'  || status === 'error';
  const isPending = status === 'pending';

  const kanjiColor = isReady   ? 'var(--success)'
                   : isBlocked ? 'var(--accent)'
                   : isBusy    ? 'var(--ink-2)'
                   : 'var(--ink-4)';

  return (
    <div className="splash-ink" style={{
      animationDelay: `${delay}s`,
      padding: '5px 0',
      borderBottom: '1px solid var(--edge)',
      opacity: isPending ? 0.5 : 1,
      transition: 'opacity .3s',
    }}>
      <div style={{
        display: 'grid', gridTemplateColumns: '22px 1fr auto',
        alignItems: 'center', gap: 12,
      }}>
        <div className="kanji" style={{
          fontSize: 18, color: kanjiColor, textAlign: 'center', lineHeight: 1,
        }}>{gate.n}</div>

        <div style={{ minWidth: 0, display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 6, flexWrap: 'wrap', lineHeight: 1.15 }}>
            <span style={{ fontFamily: 'var(--font-ui)', fontSize: 12, fontWeight: 500, color: 'var(--ink)' }}>
              {gate.name}
            </span>
            <span style={{ fontSize: 10.5, color: 'var(--ink-4)' }}>· {gate.detail}</span>
          </div>
          {gate.zen && (
            <div style={{
              fontSize: 10.5, color: 'var(--ink-3)',
              fontStyle: 'italic', lineHeight: 1.35,
            }}>
              {gate.zen}
            </div>
          )}
        </div>

        <SplashStatusIndicator status={status}/>
      </div>
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────
   SPLASH · the dialog itself (880×480 native)
   ───────────────────────────────────────────────────────────── */

function splashCopyFor(state) {
  switch (state) {
    case 'probing': return {
      eyebrow: 'starting',
      head: <>Checking the <span style={{ color: 'var(--accent)' }}>foundation.</span></>,
      sub:  'A quick health check before opening the observatory.',
    };
    case 'auto-fixing': return {
      eyebrow: 'setting up',
      head: <>Putting the room <span style={{ color: 'var(--accent)' }}>in order.</span></>,
      sub:  <>Running <span className="mono" style={{ color: 'var(--ink-2)' }}>brew bundle</span> with the manifest from <span className="mono" style={{ color: 'var(--ink-2)' }}>sensei-hq/homebrew-tap</span>. No input needed.</>,
    };
    case 'manual': return {
      eyebrow: 'needs your hand',
      head: <>One last <span style={{ color: 'var(--accent)' }}>step.</span></>,
      sub:  <>Homebrew isn't here yet. Run the script — it installs Homebrew, then everything else.</>,
    };
    case 'all-green': return {
      eyebrow: 'ready',
      head: <>The foundation <span style={{ color: 'var(--success)' }}>holds.</span></>,
      sub:  '12 projects · 1,284 memories · daemon listening. Opening the observatory.',
    };
  }
}

function Splash({ state }) {
  const statuses = SPLASH_STATE_STATUSES[state];
  const showChecks = state !== 'all-green';
  const c = splashCopyFor(state);

  return (
    <div className="sensei splash-window" data-screen-label={`Splash · ${state}`}>
      <SplashChrome/>

      {/* Watermark — only in the all-green case. Uses the logo as a
          mask so it picks up the current ink color (works in both
          light and dark themes). Anchored mid-right of the dialog. */}
      {state === 'all-green' && (
        <div aria-hidden="true"
             style={{
               position: 'absolute',
               right: 28,
               top: '50%',
               transform: 'translateY(-50%)',
               width: 260, height: 260,
               background: 'var(--ink)',
               WebkitMaskImage: 'url(site/sensei-logo.svg)',
               maskImage: 'url(site/sensei-logo.svg)',
               WebkitMaskSize: 'contain',
               maskSize: 'contain',
               WebkitMaskRepeat: 'no-repeat',
               maskRepeat: 'no-repeat',
               WebkitMaskPosition: 'center',
               maskPosition: 'center',
               opacity: 0.10,
               pointerEvents: 'none',
               userSelect: 'none',
             }}/>
      )}

      <div style={{
        flex: 1,
        display: 'grid',
        gridTemplateColumns: showChecks ? '1fr 1px 1.05fr' : '1fr',
        gap: showChecks ? 28 : 0,
        padding: '18px 32px 22px',
        alignItems: 'stretch',
        minHeight: 0,
      }}>
        {/* ── Left · identity + headline ─── */}
        <div style={{
          display: 'flex', flexDirection: 'column',
          justifyContent: 'space-between',
          minWidth: 0,
        }}>
          <div className="splash-ink">
            <SplashWordmark size={showChecks ? 'md' : 'lg'}/>
            <div style={{
              fontSize: 10.5, letterSpacing: '0.22em', color: 'var(--ink-3)',
              textTransform: 'uppercase', fontWeight: 500, marginTop: showChecks ? 18 : 26,
            }}>
              {c.eyebrow}
            </div>
            <div className="display" style={{
              fontSize: showChecks ? 26 : 34,
              fontWeight: 300, letterSpacing: '-0.02em',
              color: 'var(--ink)', marginTop: 8, lineHeight: 1.12,
            }}>
              {c.head}
            </div>
            <div style={{
              fontSize: 12.5, color: 'var(--ink-3)', lineHeight: 1.6,
              marginTop: 10, maxWidth: showChecks ? 280 : 360,
            }}>
              {c.sub}
            </div>

            {/* Manual state · remedy script + buttons */}
            {state === 'manual' && (
              <div style={{ marginTop: 14 }}>
                <pre className="splash-script">{`/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile
sensei db:create && sensei daemon:start`}</pre>
                <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
                  <button style={{
                    fontSize: 11, padding: '5px 10px',
                    background: 'var(--ink)', color: 'var(--paper)',
                    border: 'none', borderRadius: 'var(--radius-sm)', cursor: 'pointer',
                    letterSpacing: 0.2,
                  }}>Copy</button>
                  <button style={{
                    fontSize: 11, padding: '5px 10px',
                    background: 'transparent', color: 'var(--accent)',
                    border: '1px solid var(--accent)', borderRadius: 'var(--radius-sm)', cursor: 'pointer',
                  }}>I've run it · re-check</button>
                </div>
              </div>
            )}

            {/* compact handoff indicator in the green case */}
            {state === 'all-green' && (
              <div style={{
                marginTop: 22, display: 'flex', alignItems: 'center', gap: 10,
                fontSize: 11, color: 'var(--ink-3)',
              }}>
                <div className="splash-tickle" style={{
                  height: 2, width: 80, background: 'var(--success)', borderRadius: 1,
                }}/>
                <span className="mono" style={{ letterSpacing: 0.2 }}>opening…</span>
              </div>
            )}
          </div>

          {/* footer meta */}
          <div style={{
            display: 'flex', alignItems: 'center', gap: 8,
            fontSize: 10, fontFamily: 'var(--font-mono)',
            color: 'var(--ink-4)', letterSpacing: 0.3,
            marginTop: showChecks ? 16 : 28,
          }}>
            <span>sensei 0.1.0</span>
            <span style={{ width: 3, height: 3, borderRadius: '50%', background: 'var(--ink-4)' }}/>
            <span>macOS 14.4 · arm64</span>
            <span style={{ width: 3, height: 3, borderRadius: '50%', background: 'var(--ink-4)' }}/>
            <span>16gb · 412gb free</span>
            {state === 'all-green' && (
              <>
                <span style={{ width: 3, height: 3, borderRadius: '50%', background: 'var(--ink-4)' }}/>
                <span>session 247</span>
              </>
            )}
          </div>
        </div>

        {showChecks && <div style={{ background: 'var(--edge)' }}/>}

        {showChecks && (
          <div className="splash-reveal" style={{
            display: 'flex', flexDirection: 'column',
            minWidth: 0, overflow: 'hidden',
          }}>
            {/* Hero status */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <SplashStatusDisc status={splashOverallStatus(statuses)} size={32}/>
              <div style={{ minWidth: 0 }}>
                <div style={{
                  fontSize: 10.5, letterSpacing: '0.22em', color: 'var(--ink-3)',
                  textTransform: 'uppercase', fontWeight: 500, display: 'flex', alignItems: 'baseline', gap: 6,
                }}>
                  <span className="kanji" style={{
                    fontSize: 13, color: 'var(--accent)', letterSpacing: 0,
                    textTransform: 'none', lineHeight: 1,
                  }}>支</span>
                  <span>foundation</span>
                </div>
                <div style={{
                  fontFamily: 'var(--font-ui)',
                  fontSize: 16, fontWeight: 400, color: 'var(--ink-3)',
                  marginTop: 3, lineHeight: 1.25,
                  letterSpacing: '-0.005em',
                }}>
                  {(() => {
                    const readyCount = SPLASH_GATES.filter(g => statuses[g.id] === 'ready').length;
                    const o = splashOverallStatus(statuses);
                    const V = ({ children }) => (
                      <span style={{ color: 'var(--ink)', fontWeight: 500 }}>{children}</span>
                    );
                    if (o === 'ready')   return <>The foundation <V>holds.</V></>;
                    if (o === 'missing') return <>One component <V>needs your hand.</V></>;
                    if (o === 'installing') return <><V>Installing</V> · {readyCount} of {SPLASH_GATES.length} ready.</>;
                    if (o === 'starting')   return <><V>Starting</V> · {readyCount} of {SPLASH_GATES.length} ready.</>;
                    if (o === 'checking')   return <><V>Checking</V> each component.</>;
                    return <><V>Waiting</V> to start.</>;
                  })()}
                </div>
              </div>
            </div>

            {/* Ledger */}
            <div style={{
              display: 'flex', flexDirection: 'column',
              borderTop: '1px solid var(--edge)',
              marginTop: 24,
              overflow: 'auto',
              minHeight: 0, flex: 1,
            }}>
              {SPLASH_GATES.map((g, i) => (
                <SplashGateRow key={g.id}
                               gate={g}
                               status={statuses[g.id]}
                               delay={0.05 + i * 0.04}/>
              ))}
            </div>

            {/* Continue */}
            <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 14 }}>
              <button style={{
                fontSize: 12, padding: '7px 14px',
                background: 'var(--ink)', color: 'var(--paper)',
                border: 'none', borderRadius: 'var(--radius-sm)',
                cursor: 'pointer', letterSpacing: 0.2,
                fontFamily: 'inherit',
              }}>Continue →</button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/* SplashOnDesktop — wrap the 880×480 dialog as a floating window
   on a neutral washi background so it can live inside a larger
   Observatory artboard the way a real macOS startup screen would. */
function SplashOnDesktop({ state }) {
  return (
    <div className="splash-desktop">
      <div className="splash-window-frame">
        <Splash state={state}/>
      </div>
    </div>
  );
}

/* Expose to global scope for other Babel script blocks */
Object.assign(window, {
  Splash,
  SplashOnDesktop,
  SPLASH_GATES,
  SPLASH_STATE_STATUSES,
});
