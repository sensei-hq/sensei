// Startup splash · two-column health-check — SHARED MODULE
// Lifted verbatim out of the old "Sensei Splash" standalone canvas so the
// exploration lives in the Experiments board instead of a root file.
// Wrapped in an IIFE so its many primitive names (Spinner, GateRow,
// StatusDisc, Wordmark, GATES…) don't collide with the bootstrap modules'
// globals. Exposes one symbol: window.SplashHealthCheck.

(function () {
const { useState, useEffect } = React;

    /* ─────────────────────────────────────────────────────────────
       MODEL
       Same six gates as the original Bootstrap page, but the splash
       hides them entirely when everything's green — the default
       launch is just identity + a "listening" line. The right
       column reveals only when sensei has work to do or report.
       ───────────────────────────────────────────────────────────── */

    const GATES = [
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

    // statuses per overall state
    const STATE_STATUSES = {
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

    function Wordmark({ size = 'md' }) {
      const k = size === 'lg' ? 32 : size === 'sm' ? 18 : 26;
      const w = size === 'lg' ? 26 : size === 'sm' ? 14 : 20;
      return (
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
          <span className="kanji" style={{ fontSize: k, color: 'var(--accent)', lineHeight: 1 }}>先生</span>
          <span className="display" style={{ fontSize: w, fontWeight: 400, letterSpacing: '-0.01em' }}>Sensei</span>
        </div>
      );
    }

    // Spinner used in checking/starting status
    function Spinner({ size = 10, color = 'currentColor' }) {
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

    // Animated status icon — used per row AND as the big hero at the top
    // of the right column. Same vocabulary, two sizes.
    function StatusDisc({ status, size = 20 }) {
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

    // Small status label paired with a StatusDisc on each gate row.
    function StatusIndicator({ status }) {
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
        <div style={{
          display: 'inline-flex', alignItems: 'center', gap: 8, flexShrink: 0,
        }}>
          {label && (
            <span className="mono" style={{
              fontSize: 10, letterSpacing: '0.12em', textTransform: 'uppercase',
              color, lineHeight: 1,
            }}>{label}</span>
          )}
          <StatusDisc status={status} size={20}/>
        </div>
      );
    }

    // Roll up per-gate statuses into one overall status for the hero disc.
    function overallStatus(statuses) {
      const vals = Object.values(statuses);
      if (vals.some(s => s === 'missing' || s === 'error')) return 'missing';
      if (vals.some(s => s === 'installing'))               return 'installing';
      if (vals.some(s => s === 'starting'))                 return 'starting';
      if (vals.some(s => s === 'checking'))                 return 'checking';
      if (vals.every(s => s === 'ready'))                   return 'ready';
      return 'pending';
    }

    function StatusPill({ status }) {
      const map = {
        ready:    { label: 'ready',    color: 'var(--success)', bg: 'oklch(0.62 0.08 160 / 0.10)' },
        checking: { label: 'checking', color: 'var(--ink-2)',   bg: 'var(--paper-2)' },
        starting: { label: 'starting', color: 'var(--ink-2)',   bg: 'var(--paper-2)' },
        missing:  { label: 'missing',  color: 'var(--accent)',  bg: 'oklch(0.58 0.15 35 / 0.08)' },
        error:    { label: 'blocked',  color: 'var(--accent)',  bg: 'oklch(0.58 0.15 35 / 0.08)' },
        pending:  { label: 'waiting',  color: 'var(--ink-4)',   bg: 'transparent' },
      };
      const m = map[status] || map.pending;
      const isBusy = status === 'checking' || status === 'starting';
      return (
        <div style={{
          display: 'inline-flex', alignItems: 'center', gap: 5,
          padding: '2px 7px', borderRadius: 4,
          background: m.bg, color: m.color,
          fontSize: 10, fontFamily: 'var(--font-mono)',
          letterSpacing: '0.08em', textTransform: 'uppercase',
          flexShrink: 0,
        }}>
          {isBusy && <Spinner size={9}/>}
          {status === 'ready' && <span style={{ fontSize: 10 }}>✓</span>}
          {(status === 'missing' || status === 'error') && <span style={{ fontSize: 12, lineHeight: 1 }}>·</span>}
          {m.label}
        </div>
      );
    }

    // One gate row — older Bootstrap design, compressed for the splash column.
    // Kanji numeral · name · detail · check command · status pill on the right.
    // The first blocked gate inline-expands with a remedy block.
    function GateRow({ gate, status, isFirstBlocked, delay = 0 }) {
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

            <StatusIndicator status={status}/>
          </div>

          {/* Remedy now lives in the left column for manual state — see Splash */}
        </div>
      );
    }

    /* ─────────────────────────────────────────────────────────────
       SPLASH
       The all-green case is a compact single column. Anything else
       reveals the second column with the gate list.
       ───────────────────────────────────────────────────────────── */

    function copyFor(state) {
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
      const statuses = STATE_STATUSES[state];
      const readyCount = GATES.filter(g => statuses[g.id] === 'ready').length;
      const showChecks = state !== 'all-green';
      const c = copyFor(state, readyCount, GATES.length);
      const firstBlockedIdx = GATES.findIndex(g => {
        const s = statuses[g.id];
        return s === 'missing' || s === 'error';
      });

      return (
        <div className="sensei splash-window" data-screen-label={`Splash · ${state}`}>
          <SplashChrome/>

          {/* Watermark — only in the all-green case. Uses the logo as
              a CSS mask so it picks up the current ink color (works
              in both light and dark themes). */}
          {state === 'all-green' && (
            <div aria-hidden="true"
                 style={{
                   position: 'absolute',
                   right: 28,
                   top: '50%',
                   transform: 'translateY(-50%)',
                   width: 260, height: 260,
                   background: 'var(--ink)',
                   WebkitMaskImage: 'url(uploads/sensei.svg?v=2)',
                   maskImage: 'url(uploads/sensei.svg?v=2)',
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
            {/* ── Left · identity + headline (always shown) ─── */}
            <div style={{
              display: 'flex', flexDirection: 'column',
              justifyContent: 'space-between',
              minWidth: 0,
            }}>
              <div className="splash-ink">
                <Wordmark size={showChecks ? 'md' : 'lg'}/>
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

                {/* Manual state · the remedy lives here in the left column,
                    next to its explanation. The right column still shows
                    "missing" status on the blocked gate. */}
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

            {/* ── Divider ──────────────────────────────────── */}
            {showChecks && <div style={{ background: 'var(--edge)' }}/>}

            {/* ── Right · hero status + gate ledger ── */}
            {showChecks && (
              <div className="splash-reveal" style={{
                display: 'flex', flexDirection: 'column',
                minWidth: 0, overflow: 'hidden',
              }}>
                {/* Hero status — one big disc summarising the whole foundation */}
                <div style={{
                  display: 'flex', alignItems: 'center', gap: 12,
                }}>
                  <StatusDisc status={overallStatus(statuses)} size={32}/>
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
                        const readyCount = GATES.filter(g => statuses[g.id] === 'ready').length;
                        const o = overallStatus(statuses);
                        const V = ({ children }) => (
                          <span style={{ color: 'var(--ink)', fontWeight: 500 }}>{children}</span> /* medium — sits between regular and semibold */
                        );
                        if (o === 'ready')   return <>The foundation <V>holds.</V></>;
                        if (o === 'missing') return <>One component <V>needs your hand.</V></>;
                        if (o === 'installing') return <><V>Installing</V> · {readyCount} of {GATES.length} ready.</>;
                        if (o === 'starting')   return <><V>Starting</V> · {readyCount} of {GATES.length} ready.</>;
                        if (o === 'checking')   return <><V>Checking</V> each component.</>;
                        return <><V>Waiting</V> to start.</>;
                      })()}
                    </div>
                  </div>
                </div>

                {/* Ledger — small dot + name + detail + status, no kanji numerals */}
                <div style={{
                  display: 'flex', flexDirection: 'column',
                  borderTop: '1px solid var(--edge)',
                  marginTop: 24,
                  overflow: 'auto',
                  minHeight: 0, flex: 1,
                }}>
                  {GATES.map((g, i) => (
                    <GateRow key={g.id}
                             gate={g}
                             status={statuses[g.id]}
                             isFirstBlocked={i === firstBlockedIdx}
                             delay={0.05 + i * 0.04}/>
                  ))}
                </div>

                {/* Continue button — always available; the splash auto-advances
                    when foundation is ready, but the user can dismiss earlier. */}
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

  window.SplashHealthCheck = Splash;
})();
