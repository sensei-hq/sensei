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
      border-radius: 0;
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

    /* Splash desktop — the bootstrap runs as a full desktop screen
       (edge-to-edge app window), not a small floating dialog. */
    .splash-desktop {
      width: 100%; height: 100%;
      display: flex;
      background: var(--paper);
    }
    .splash-desktop > .splash-window-frame {
      width: 100%; height: 100%;
      border-radius: 0;
    }
  `;
  document.head.appendChild(s);
})();

/* ─────────────────────────────────────────────────────────────
   MODEL
   ───────────────────────────────────────────────────────────── */

const SPLASH_GATES = [
{ id: 'homebrew', n: '一', name: 'Homebrew', detail: 'package manager', check: 'which brew',
  why: 'Installs and updates everything else below from one manifest.',
  ifMissing: 'The only thing you may have to install by hand — one script does it.' },
{ id: 'postgres', n: '二', name: 'PostgreSQL', detail: 'storage · @16', check: 'brew list postgresql@16',
  why: 'The local database where every session and memory is stored.',
  ifMissing: 'Installed automatically · nothing is sent anywhere.' },
{ id: 'ollama', n: '三', name: 'Ollama', detail: 'local models', check: 'brew list ollama',
  why: 'Runs the models on-device so your code never leaves the machine.',
  ifMissing: 'Installed automatically · models pulled on first use.' },
{ id: 'sensei', n: '四', name: 'Sensei components', detail: 'cli · mcp · daemon', check: 'sensei --version',
  why: 'The CLI, the MCP server assistants talk to, and the watcher.',
  ifMissing: 'Installed automatically from the tap.' },
{ id: 'database', n: '五', name: 'Database', detail: 'schema · pgvector', check: 'sensei db:create',
  why: 'Creates the schema and vector index memories are searched through.',
  ifMissing: 'Created automatically once PostgreSQL is up.' },
{ id: 'daemon', n: '六', name: 'Daemon', detail: 'background', check: 'sensei daemon:start',
  why: 'Watches sessions in the background — nothing works without it.',
  ifMissing: 'Started automatically · restarts with your machine.' }];


const SPLASH_STATE_STATUSES = {
  probing: {
    homebrew: 'checking', postgres: 'pending', ollama: 'pending',
    sensei: 'pending', database: 'pending', daemon: 'pending'
  },
  'auto-fixing': {
    homebrew: 'ready', postgres: 'ready', ollama: 'ready',
    sensei: 'installing', database: 'pending', daemon: 'pending'
  },
  manual: {
    homebrew: 'missing', postgres: 'pending', ollama: 'pending',
    sensei: 'pending', database: 'pending', daemon: 'pending'
  },
  'all-green': {
    homebrew: 'ready', postgres: 'ready', ollama: 'ready',
    sensei: 'ready', database: 'ready', daemon: 'ready'
  }
};

/* ─────────────────────────────────────────────────────────────
   PRIMITIVES
   ───────────────────────────────────────────────────────────── */

function SplashChrome() {
  return (
    <div className="splash-chrome">
      <div className="tauri-traffic"><span /><span /><span /></div>
    </div>);

}

function SplashWordmark({ size = 'md' }) {
  const k = size === 'lg' ? 32 : size === 'sm' ? 18 : 26;
  const w = size === 'lg' ? 26 : size === 'sm' ? 14 : 20;
  return <Wordmark size={k} textSize={w}/>;
}

function SplashSpinner({ size = 10, color = 'currentColor' }) {
  return (
    <span className="inline-block relative" style={{ width: size, height: size }}>
      <span className="absolute rounded-full" style={{ inset: 0,
 border: `1.5px solid ${color}`, borderTopColor: 'transparent',
 animation: 'splashSpin 0.9s linear infinite'
 }} />
    </span>);

}

function SplashStatusDisc({ status, size = 20 }) {
  const isReady = status === 'ready';
  const isBusy = status === 'checking' || status === 'starting' || status === 'installing';
  const isBlocked = status === 'missing' || status === 'error';
  const isPending = status === 'pending';

  const color = isReady ? 'var(--success)' :
  isBlocked ? 'var(--accent)' :
  isBusy ? 'var(--accent)' :
  'var(--ink-4)';

  const inner = size >= 32 ? 14 : size >= 24 ? 11 : 10;
  const stroke = size >= 32 ? 2 : 1.5;

  return (
    <div className="rounded-full bg-paper flex items-center justify-center shrink-0" style={{
 width: size, height: size,
 border: `${stroke}px solid ${color}`,
 opacity: isPending ? 0.55 : 1,
 transition: 'opacity .25s, border-color .25s',
 boxShadow: isBusy ? `0 0 0 ${Math.round(size * 0.18)}px ${color}1f` : 'none'
 }}>
      {isReady &&
      <svg width={inner} height={inner} viewBox="0 0 10 10" fill="none">
          <path d="M2 5.2 L4.2 7.2 L8 3" stroke={color}
        strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      }
      {isBusy &&
      <span className="rounded-full" style={{
 width: inner, height: inner,
 border: `${stroke}px solid ${color}`, borderTopColor: 'transparent',
 animation: 'splashSpin 0.9s linear infinite'
 }} />
      }
      {isBlocked &&
      <span className="kanji font-normal" style={{
 fontSize: Math.round(size * 0.6), color, lineHeight: 1 }}>?</span>
      }
    </div>);

}

function SplashStatusIndicator({ status }) {
  const labelMap = {
    checking: 'checking',
    installing: 'installing',
    starting: 'starting',
    missing: 'missing',
    error: 'blocked',
    pending: null,
    ready: null
  };
  const label = labelMap[status];
  const isReady = status === 'ready';
  const isBusy = status === 'checking' || status === 'starting' || status === 'installing';
  const isBlocked = status === 'missing' || status === 'error';
  const color = isReady ? 'var(--success)' :
  isBlocked ? 'var(--accent)' :
  isBusy ? 'var(--accent)' :
  'var(--ink-4)';
  return (
    <div className="inline-flex items-center shrink-0" style={{ gap: 8 }}>
      {label &&
      <span className="mono uppercase" style={{
 fontSize: 10, letterSpacing: '0.12em',
 color, lineHeight: 1
 }}>{label}</span>
      }
      <SplashStatusDisc status={status} size={20} />
    </div>);

}

function splashOverallStatus(statuses) {
  const vals = Object.values(statuses);
  if (vals.some((s) => s === 'missing' || s === 'error')) return 'missing';
  if (vals.some((s) => s === 'installing')) return 'installing';
  if (vals.some((s) => s === 'starting')) return 'starting';
  if (vals.some((s) => s === 'checking')) return 'checking';
  if (vals.every((s) => s === 'ready')) return 'ready';
  return 'pending';
}

/* One gate row — kanji numeral · name · detail + zen line · status indicator */
function SplashGateRow({ gate, status, delay = 0 }) {
  const isReady = status === 'ready';
  const isBusy = status === 'checking' || status === 'starting' || status === 'installing';
  const isBlocked = status === 'missing' || status === 'error';
  const isPending = status === 'pending';

  const kanjiColor = isReady ? 'var(--success)' :
  isBlocked ? 'var(--accent)' :
  isBusy ? 'var(--ink-2)' :
  'var(--ink-4)';

  return (
    <div className="splash-ink" style={{
      animationDelay: `${delay}s`,
      padding: '8px 0',
      borderBottom: '1px solid var(--edge)',
      opacity: isPending ? 0.5 : 1,
      transition: 'opacity .3s'
    }}>
      <div className="grid items-center" style={{ gridTemplateColumns: '22px 1fr auto', gap: 12
 }}>
        <div className="kanji text-center" style={{
 fontSize: 18, color: kanjiColor, lineHeight: 1
 }}>{gate.n}</div>

        <div className="min-w-0 flex flex-col" style={{ gap: 4 }}>
          <div className="flex items-baseline flex-wrap" style={{ gap: 6, lineHeight: 1.15 }}>
            <span className="font-medium text-ink" style={{ fontFamily: 'var(--font-ui)', fontSize: 12 }}>
              {gate.name}
            </span>
            <span className="text-ink-4" style={{ fontSize: 10.5 }}>· {gate.detail}</span>
          </div>
          {gate.why &&
          <div className="text-ink-3" style={{
 fontSize: 10.5, lineHeight: 1.4
 }}>
              {gate.why}
            </div>
          }
          {/* What happens if this one isn't satisfied — answers the "no why" gap. */}
          {gate.ifMissing &&
          <div style={{
            fontSize: 10, lineHeight: 1.35,
            color: isBlocked ? 'var(--accent)' : 'var(--ink-4)'
          }}>
              {isBlocked ? '→ ' : ''}{gate.ifMissing}
            </div>
          }
        </div>

        <SplashStatusIndicator status={status} />
      </div>
    </div>);

}

/* ─────────────────────────────────────────────────────────────
   SPLASH · the dialog itself (880×480 native)
   ───────────────────────────────────────────────────────────── */

function splashCopyFor(state) {
  switch (state) {
    case 'probing':return {
        eyebrow: 'starting',
        head: <>Checking the <span className="text-accent" >foundation.</span></>,
        sub: 'A quick health check before opening the observatory.'
      };
    case 'auto-fixing':return {
        eyebrow: 'setting up',
        head: <>Putting the room <span className="text-accent" >in order.</span></>,
        sub: <>Running <span className="mono text-ink-2" >brew bundle</span> with the manifest from <span className="mono text-ink-2" >sensei-hq/homebrew-tap</span>. No input needed.</>
      };
    case 'manual':return {
        eyebrow: 'needs your hand',
        head: <>One last <span className="text-accent" >step.</span></>,
        sub: <>Homebrew isn't here yet. Run the script — it installs Homebrew, then everything else.</>
      };
    case 'all-green':return {
        eyebrow: 'ready',
        head: <>The foundation <span className="text-success" >holds.</span></>,
        sub: '12 projects · 1,284 memories · daemon listening. Opening the observatory.'
      };
  }
}

function Splash({ state }) {
  const statuses = SPLASH_STATE_STATUSES[state];
  const showChecks = true; // every state uses the two-column layout, incl. all-green
  const c = splashCopyFor(state);

  return (
    <div className="sensei splash-window" data-screen-label={`Splash · ${state}`}>
      <SplashChrome />


      <div className="flex-1 flex items-center justify-center min-h-0" style={{ padding: 48
 }}>
      <div className="w-full h-full grid items-stretch min-h-0" style={{
 maxWidth: showChecks ? 1000 : 720, maxHeight: 520,
 gridTemplateColumns: showChecks ? '1fr 1px 1.05fr' : '1fr',
 gap: showChecks ? 28 : 0 }}>
        {/* ── Left · identity + headline ─── */}
        <div className="flex flex-col justify-between min-w-0 min-h-0" >
          <div className="splash-ink flex flex-col flex-1 min-h-0" >
            <SplashWordmark size={showChecks ? 'md' : 'lg'} />
            <div className="text-ink-3 uppercase font-medium" style={{
 fontSize: 10.5, letterSpacing: '0.22em', marginTop: showChecks ? 18 : 26
 }}>
              {c.eyebrow}
            </div>
            <div className="display font-light text-ink" style={{
 fontSize: showChecks ? 26 : 34, letterSpacing: '-0.02em', marginTop: 8, lineHeight: 1.12
 }}>
              {c.head}
            </div>
            <div className="text-ink-3" style={{
 fontSize: 12.5, lineHeight: 1.6,
 marginTop: 10, maxWidth: showChecks ? 280 : 360
 }}>
              {c.sub}
            </div>

            {/* Probing / auto-fixing · framing card — answers "what is all this,
                and what happens if something's missing" while the ledger runs.
                Fills the otherwise-empty left column in these states. */}
            {state === 'probing' &&
              <div className="bg-paper-2" style={{
 marginTop: 18, maxWidth: 300,
 border: '1px solid var(--edge)', borderRadius: 8,
 padding: '12px 16px' }}>
                <div className="uppercase text-ink-4 font-medium" style={{
 fontSize: 10, letterSpacing: '0.16em', marginBottom: 6
 }}>
                  What this is
                </div>
                <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.55 }}>
                  Sensei runs on a small local foundation — a database, a model
                  runtime, and a background watcher. This check confirms each
                  piece is present before opening.
                </div>
                <div className="flex items-start" style={{ gap: 8, marginTop: 10, paddingTop: 10,
 borderTop: '1px solid var(--edge)' }}>
                  <span className="kanji text-accent" style={{ fontSize: 14, lineHeight: 1.2 }}>修</span>
                  <div className="text-ink-3" style={{ fontSize: 11, lineHeight: 1.5 }}>
                    Anything missing is installed for you — usually automatically,
                    occasionally with one script to paste. Nothing leaves your machine.
                  </div>
                </div>
              </div>
              }

            {/* Auto-fixing · progress panel — install cost (time + size) and a
                live activity line, so a long install reads as motion, not a hang. */}
            {state === 'auto-fixing' &&
              <div className="bg-paper-2" style={{
 marginTop: 18, maxWidth: 320,
 border: '1px solid var(--edge)', borderRadius: 8,
 padding: '16px 16px' }}>
                <div className="flex items-baseline justify-between" style={{ marginBottom: 10 }}>
                  <span className="uppercase text-ink-4 font-medium" style={{ fontSize: 10, letterSpacing: '0.16em' }}>Installing</span>
                  <span className="mono text-ink-3" style={{ fontSize: 10.5 }}>≈ 2 min left</span>
                </div>
                <div className="bg-paper-3 overflow-hidden" style={{ height: 4, borderRadius: 2 }}>
                  <div className="h-full bg-accent" style={{ width: '58%', borderRadius: 2 }} />
                </div>
                <div className="flex justify-between" style={{ marginTop: 6 }}>
                  <span className="mono text-ink-4" style={{ fontSize: 10 }}>3 of 6 ready</span>
                  <span className="mono text-ink-4" style={{ fontSize: 10 }}>1.4 / 2.1 GB</span>
                </div>
                <div className="flex items-center" style={{ gap: 8, marginTop: 12,
 paddingTop: 12, borderTop: '1px solid var(--edge)' }}>
                  <SplashSpinner size={11} color="var(--accent)" />
                  <span className="mono splash-tickle text-ink-2 whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 10.5 }}>
                    ==&gt; Pouring ollama--0.5.1.arm64.bottle.tar.gz
                  </span>
                </div>
                <div className="text-ink-4" style={{ fontSize: 10.5, lineHeight: 1.45, marginTop: 10 }}>
                  Runs once · future launches skip straight through. Nothing leaves your machine.
                </div>
              </div>
              }

            {/* Manual state · guided step-by-step — the one hand-run action, then
                what Sensei does automatically, plus an offline / proxy path. */}
            {state === 'manual' &&
              <div className="flex-1 min-h-0 flex flex-col" style={{
 marginTop: 14, gap: 10
 }}>
                {/* Step 1 — the only manual action */}
                <div className="overflow-hidden" style={{ border: '1px solid var(--edge)', borderRadius: 8 }}>
                  <div className="flex items-center" style={{ gap: 10,
 padding: '12px 16px', borderBottom: '1px solid var(--edge)' }}>
                    <span className="rounded-full bg-accent text-paper font-semibold flex items-center justify-center shrink-0" style={{ width: 18, height: 18, fontSize: 11 }}>1</span>
                    <div className="flex-1 min-w-0" >
                      <div className="font-semibold text-ink" style={{ fontSize: 12.5 }}>Install Homebrew</div>
                      <div className="text-ink-4" style={{ fontSize: 10.5 }}>The one thing to run by hand · ~1 min</div>
                    </div>
                    <button className="bg-ink text-paper border-0 rounded-sm cursor-pointer" style={{ fontSize: 11, padding: '8px 12px', letterSpacing: 0.2 }}>Copy</button>
                  </div>
                  <pre className="splash-script" style={{ borderRadius: 0, padding: '12px 16px', maxHeight: 'none' }}>{`/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"`}</pre>
                </div>

                {/* Step 2 — automatic */}
                <div className="flex items-start" style={{ border: '1px solid var(--edge)', borderRadius: 8, padding: '12px 16px', gap: 10 }}>
                  <span className="rounded-full bg-paper-3 text-ink-3 font-semibold flex items-center justify-center shrink-0" style={{ width: 18, height: 18, fontSize: 11 }}>2</span>
                  <div className="flex-1" >
                    <div className="font-semibold text-ink" style={{ fontSize: 12.5 }}>Sensei takes it from here</div>
                    <div className="text-ink-3" style={{ fontSize: 10.5, lineHeight: 1.5, marginTop: 2 }}>
                      The moment Homebrew is in, PostgreSQL · Ollama · components · the database · the daemon install on their own. No more commands.
                    </div>
                  </div>
                </div>

                {/* Offline / proxy path */}
                <div style={{ border: '1px dashed var(--edge)', borderRadius: 8, padding: '12px 12px' }}>
                  <div className="uppercase text-ink-4 font-medium" style={{ fontSize: 10, letterSpacing: '0.14em', marginBottom: 5 }}>No internet · behind a proxy</div>
                  <div className="text-ink-3" style={{ fontSize: 10.5, lineHeight: 1.5 }}>
                    Set <span className="mono text-ink-2" >HTTPS_PROXY</span> first, or grab the offline bundle — both install the same foundation without a live connection.
                  </div>
                  <a className="text-accent cursor-pointer inline-block no-underline" style={{ fontSize: 10.5, marginTop: 6 }}>Offline install guide →</a>
                </div>

                {/* Re-check */}
                <div className="flex items-center justify-end" style={{ gap: 8, marginTop: 'auto' }}>
                  <button className="bg-accent text-paper border-0 rounded-sm cursor-pointer" style={{ fontSize: 11.5, padding: '8px 12px', letterSpacing: 0.2 }}>
                    I've run it · re-check
                  </button>
                </div>
              </div>
              }

            {/* All-green · watermark logo fills the empty left-column space */}
            {state === 'all-green' &&
              <div className="flex-1 relative" aria-hidden="true" style={{ minHeight: 80, marginTop: 18
 }}>
                <div className="absolute bg-ink" style={{ inset: 0,
 WebkitMaskImage: 'url(uploads/sensei.svg?v=3)',
 maskImage: 'url(uploads/sensei.svg?v=3)',
 WebkitMaskSize: '62%', maskSize: '62%',
 WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
 WebkitMaskPosition: 'center', maskPosition: 'center',
 opacity: 0.08, pointerEvents: 'none', userSelect: 'none'
 }} />
              </div>
              }

            {/* compact handoff indicator in the green case */}
            {state === 'all-green' &&
              <div className="flex items-center text-ink-3" style={{
 marginTop: 22, gap: 10,
 fontSize: 11 }}>
                <div className="splash-tickle bg-success" style={{
 height: 2, width: 80, borderRadius: 1
 }} />
                <span className="mono" style={{ letterSpacing: 0.2 }}>opening…</span>
              </div>
              }
          </div>

          {/* footer meta */}
          <div className="flex items-center text-ink-4" style={{ gap: 8,
 fontSize: 10, fontFamily: 'var(--font-mono)', letterSpacing: 0.3,
 marginTop: showChecks ? 16 : 28
 }}>
            <span>sensei 0.1.0</span>
            <span className="rounded-full bg-ink-4" style={{ width: 3, height: 3 }} />
            <span>macOS 14.4 · arm64</span>
            <span className="rounded-full bg-ink-4" style={{ width: 3, height: 3 }} />
            <span>16gb · 412gb free</span>
            {state === 'all-green' &&
              <>
                <span className="rounded-full bg-ink-4" style={{ width: 3, height: 3 }} />
                <span>session 247</span>
              </>
              }
          </div>
        </div>

        {showChecks && <div style={{ background: 'var(--edge)' }} />}

        {showChecks &&
          <div className="splash-reveal flex flex-col min-w-0 overflow-hidden" >
            {/* Hero status — standard KanjiHeader (kanji · eyebrow · title · right) */}
            {(() => {
              const total = SPLASH_GATES.length;
              const readyCount = SPLASH_GATES.filter((g) => statuses[g.id] === 'ready').length;
              const o = splashOverallStatus(statuses);
              const title =
              o === 'ready' ? 'All systems ready' :
              o === 'missing' ? 'Needs your hand' :
              o === 'installing' ? `Installing · ${readyCount}/${total}` :
              o === 'starting' ? `Starting · ${readyCount}/${total}` :
              o === 'checking' ? 'Checking components' :
              'Waiting to start';
              return (
                <KanjiHeader
                  kanji="支"
                  eyebrow="foundation"
                  title={title}
                  right={
                    <div className="flex items-center" style={{ gap: 10 }}>
                      <button className="inline-flex items-center text-ink-3 bg-transparent rounded-sm cursor-pointer" title="Re-run the foundation check and repair anything missing"
 style={{ gap: 5, fontSize: 10.5,
 fontFamily: 'var(--font-ui)',
 border: '1px solid var(--edge)',
 padding: '4px 12px', letterSpacing: 0.2 }}>
                        <span style={{ fontSize: 11, lineHeight: 1 }}>↻</span> Re-check
                      </button>
                      <SplashStatusDisc status={o} size={32} />
                    </div>
                  } />);

            })()}

            {/* Ledger */}
            <div className="flex flex-col overflow-auto min-h-0 flex-1" style={{
 borderTop: '1px solid var(--edge)',
 marginTop: 24 }}>
              {SPLASH_GATES.map((g, i) =>
              <SplashGateRow key={g.id}
              gate={g}
              status={statuses[g.id]}
              delay={0.05 + i * 0.04} />
              )}
            </div>

            {/* Continue */}
            <div className="flex justify-end" style={{ marginTop: 14 }}>
              <button className="bg-ink text-paper border-0 rounded-sm cursor-pointer" style={{
 fontSize: 12, padding: '8px 16px', letterSpacing: 0.2,
 fontFamily: 'inherit'
 }}>Continue →</button>
            </div>
          </div>
          }
      </div>
      </div>
    </div>);

}

/* SplashOnDesktop — the bootstrap runs as a full desktop screen
   (edge-to-edge app window) inside the Observatory artboard, with
   its content centered and margined rather than a floating dialog. */
function SplashOnDesktop({ state }) {
  return (
    <div className="splash-desktop">
      <div className="splash-window-frame">
        <Splash state={state} />
      </div>
    </div>);

}

/* Expose to global scope for other Babel script blocks */
Object.assign(window, {
  Splash,
  SplashOnDesktop,
  SPLASH_GATES,
  SPLASH_STATE_STATUSES
});