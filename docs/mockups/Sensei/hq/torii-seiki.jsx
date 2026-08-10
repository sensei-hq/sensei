// ─────────────────────────────────────────────────────────────────
// TORII + SEIKI — product page (Sensei HQ site)
// Two clients for an org's models, both built on Gateway:
// Torii (member workspace) and Seiki (admin portal).
// Same Zen-Sumi world as the HQ site: washi paper, sumi ink,
// Fraunces display, kanji marks, hairlines, one rationed accent.
// ─────────────────────────────────────────────────────────────────

const { useEffect: hE } = React;
const { TweaksPanel, useTweaks, TweakSection, TweakToggle, TweakRadio } = window;

const MAXW = 1120;

const ACCENT = {
  light: { torii: 'oklch(0.560 0.140 15)', seiki: 'oklch(0.560 0.130 255)' },
  dark:  { torii: 'oklch(0.700 0.140 15)', seiki: 'oklch(0.700 0.130 255)' },
};

const GATEWAY_URL = 'https://gateway.sensei-hq.com';

const CLIENTS = [
  {
    id: 'torii', kanji: '門', gloss: 'Mon · the gate',
    name: 'Torii', category: 'Desktop · Member workspace',
    tagline: 'The gate your team walks through.',
    blurb: 'Everything a member needs and nothing they don’t. Ask a model, keep a library, try things in the playground — and always see where the answer actually ran.',
    surfaces: ['Workspace', 'Ask', 'Library', 'Playground', 'Activity', 'Settings'],
    meta: ['macOS · Windows · Linux', 'Tauri', 'Works offline'],
    status: 'Beta',
  },
  {
    id: 'seiki', kanji: '社', gloss: 'Sha · the sanctuary',
    name: 'Seiki', category: 'Web · Governance plane',
    tagline: 'The sanctuary behind it.',
    blurb: 'The quiet room where the rules are kept. Every request, every provider, every fallback chain and every budget — set once, cascading down your org, without standing over anyone’s shoulder.',
    surfaces: ['Overview', 'Requests & audit', 'Organization', 'Models', 'Routing', 'Connections', 'Governance', 'Budgets'],
    meta: ['Self-hosted', 'SSO · SCIM', 'Full audit trail'],
    status: 'Beta',
  },
];

const CAPABILITIES = [
  { kanji: '路', title: 'Fallback chains', text: 'Order your models once — primary, then the next, then the local one. Gateway walks the chain when a provider stalls, and Seiki shows you how often it had to.' },
  { kanji: '繋', title: 'Connections', text: 'Bring your own keys for every provider and router. Strategos holds the credentials; your apps hold one address.' },
  { kanji: '具', title: 'MCP & tools', text: 'Register MCP servers — stdio on the desktop, http for shared ones — and allow-list tools per role and space.' },
  { kanji: '鍵', title: 'Programmatic access', text: 'Scoped keys turn the same gateway into an endpoint your own services can call, with usage attributed back to a team.' },
  { kanji: '階', title: 'Hierarchy & budgets', text: 'Org, department, team, person — your real structure. Permissions follow it, and so do spend caps, cascading downward.' },
  { kanji: '器', title: 'Devices & offline', text: 'Local models run on the machine in front of you. When the gateway is unreachable, work continues and syncs later.' },
];

const PLANES = [
  { label: 'On your device', kanji: '手', text: 'Local models, local context, nothing leaves the machine. Marked as such on every answer.' },
  { label: 'Via the gateway', kanji: '関', text: 'Routed, logged and budgeted through your own deployment — in the region you chose.' },
];

// ─── Primitives ──────────────────────────────────────────────────────
function Enso({ size = 26, stroke = 'var(--accent)' }) {
  return (
    <span className="block shrink-0" aria-hidden="true"
 style={{ width: size, height: size, background: stroke,
 WebkitMaskImage: 'url(uploads/sensei.svg?v=3)', maskImage: 'url(uploads/sensei.svg?v=3)',
 WebkitMaskSize: 'contain', maskSize: 'contain',
 WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
 WebkitMaskPosition: 'center', maskPosition: 'center' }} />
  );
}

function Eyebrow({ children, style }) {
  return <div className="zs-eyebrow" style={style}>{children}</div>;
}

function MetaChips({ meta }) {
  return (
    <div className="gap-2 flex flex-wrap">
      {meta.map((m) => (
        <span key={m} className="mono text-ink-mute border border-paper-edge rounded-sm whitespace-nowrap"
 style={{ fontSize: 11, padding: '2px 7px' }}>{m}</span>
      ))}
    </div>
  );
}

function SectionHead({ kanji, eyebrow, title, sub }) {
  return (
    <div className="mb-12">
      <div className="gap-3 mb-4 flex items-baseline">
        <span className="kanji text-accent" style={{ fontSize: 28, lineHeight: 1 }}>{kanji}</span>
        <Eyebrow>{eyebrow}</Eyebrow>
      </div>
      <h2 className="display text-ink m-0 font-light"
 style={{ fontSize: 40, lineHeight: 1.15, letterSpacing: '-0.022em', maxWidth: 720 }}>{title}</h2>
      {sub && <p className="text-ink-soft mt-4 m-0" style={{ fontSize: 17, lineHeight: 1.6, maxWidth: 560 }}>{sub}</p>}
    </div>
  );
}

// ─── Nav ─────────────────────────────────────────────────────────────
function Nav() {
  const links = [['#clients', 'The pair'], ['#planes', 'Two planes'], ['#capabilities', 'Capabilities'], ['#gateway', 'Gateway']];
  return (
    <div className="sticky" style={{ top: 0, zIndex: 50,
 background: 'color-mix(in oklch, var(--paper) 80%, transparent)',
 backdropFilter: 'blur(14px) saturate(150%)', WebkitBackdropFilter: 'blur(14px) saturate(150%)',
 WebkitMaskImage: 'linear-gradient(to bottom, #000 72%, transparent)',
 maskImage: 'linear-gradient(to bottom, #000 72%, transparent)', paddingBottom: 6 }}>
      <nav style={{ maxWidth: MAXW }} className="mx-auto px-12 py-4 flex items-center justify-between">
        <a href="Sensei HQ.html" className="gap-3 flex items-center">
          <Enso size={26} stroke="var(--accent)" />
          <span className="gap-2 flex items-baseline">
            <span className="display text-ink" style={{ fontSize: 17, letterSpacing: '-0.01em' }}>Torii · Seiki</span>
            <span className="mono text-ink-mute" style={{ fontSize: 11, letterSpacing: '0.08em' }}>SENSEI HQ</span>
          </span>
        </a>
        <div className="gap-8 flex items-center">
          {links.map(([href, label]) => (
            <a key={href} href={href} className="text-ink-soft text-sm" style={{ transition: 'color .15s' }}
               onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
               onMouseLeave={(e) => e.currentTarget.style.color = 'var(--ink-soft)'}>{label}</a>
          ))}
          <a href="#contact" className="zs-btn zs-btn-primary zs-btn-sm" style={{ marginLeft: 4 }}>Request access</a>
        </div>
      </nav>
    </div>
  );
}

// ─── Hero ────────────────────────────────────────────────────────────
function Hero() {
  return (
    <header id="top" style={{ maxWidth: MAXW }} className="mx-auto px-12 pt-12 pb-24">
      <div style={{ gridTemplateColumns: '1.55fr 1fr' }} className="gap-16 grid items-start">
        <div>
          <div className="gap-3 mb-6 flex items-baseline">
            <span className="kanji text-accent" style={{ fontSize: 44, lineHeight: 1 }}>門</span>
            <Eyebrow>Torii · Seiki — the gate and what stands behind it</Eyebrow>
          </div>
          <h1 className="display text-ink m-0 font-light"
 style={{ fontSize: 56, lineHeight: 1.08, letterSpacing: '-0.025em', maxWidth: 640 }}>
The gate, and the sanctuary behind it.
          </h1>
          <p className="text-ink-soft mt-6 m-0" style={{ fontSize: 17, lineHeight: 1.6, maxWidth: 540 }}>
            Torii is the gate your team walks through to reach your models. Seiki is the
            governance plane behind it — routing, budgets, audit. Both stand on Gateway,
            our Rust library for fallback chains and budget control.
          </p>
          <div className="gap-3 mt-8 flex items-center flex-wrap">
            <a href="#contact" className="zs-btn zs-btn-primary zs-btn-lg">
              <span className="kanji text-on-primary" style={{ fontSize: 15, lineHeight: 1 }}>入</span>
              Request access
            </a>
            <a href="#clients" className="text-ink-soft text-sm">Meet Torii and Seiki ↓</a>
          </div>
          <div className="gap-4 mt-12 flex flex-wrap">
            {['Self-hosted', 'Bring your own keys', 'Built on Gateway'].map((m) => (
              <span key={m} className="mono text-ink-mute" style={{ fontSize: 11 }}>{m}</span>
            ))}
          </div>
        </div>

        <aside className="border border-paper-edge rounded-lg bg-paper-soft overflow-hidden">
          <div className="px-6 py-4 border-b border-paper-edge flex items-center justify-between">
            <Eyebrow>The pair</Eyebrow>
            <span className="mono text-ink-faint" style={{ fontSize: 11 }}>02</span>
          </div>
          <div className="divide-y">
            {CLIENTS.map((c) => (
              <a key={c.id} href={'#' + c.id}
 style={{ gridTemplateColumns: 'auto 1fr auto', '--accent': `var(--acc-${c.id})`, transition: 'background .15s' }}
 className="gap-4 px-6 py-4 grid items-center"
 onMouseEnter={(e) => e.currentTarget.style.background = 'var(--paper-mute)'}
 onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                <span className="kanji text-accent text-center" style={{ fontSize: 26, lineHeight: 1, width: 30 }}>{c.kanji}</span>
                <span>
                  <span className="display text-ink block" style={{ fontSize: 15 }}>{c.name}</span>
                  <span className="text-ink-mute text-xs">{c.category}</span>
                </span>
                <span className="zs-dot bg-accent" />
              </a>
            ))}
          </div>
          <div className="px-6 py-4 border-t border-paper-edge">
            <p className="text-ink-mute m-0" style={{ fontSize: 13, lineHeight: 1.6 }}>
Pass through the gate and the rules are already in force. That is the point of the pair.
            </p>
          </div>
        </aside>
      </div>
    </header>
  );
}

// ─── Two planes ──────────────────────────────────────────────────────
function Planes() {
  return (
    <section id="planes" className="sec border-t border-paper-edge">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12">
        <SectionHead kanji="分" eyebrow="Bun · to divide" title="Every answer knows where it ran."
          sub="The split execution plane is not a setting buried in preferences — it is written on the response, every time." />
        <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-6 grid">
          {PLANES.map((p) => (
            <div key={p.label} className="zs-card">
              <div className="gap-3 mb-4 flex items-center">
                <span className="kanji text-accent" style={{ fontSize: 22, lineHeight: 1 }}>{p.kanji}</span>
                <span className="mono text-ink border border-paper-edge rounded-full" style={{ fontSize: 11, padding: '3px 10px' }}>{p.label}</span>
              </div>
              <p className="text-ink-soft m-0" style={{ fontSize: 15, lineHeight: 1.65 }}>{p.text}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── Clients ─────────────────────────────────────────────────────────
function ClientCard({ c, flip }) {
  const panel = (
    <div className="flex items-center justify-center bg-paper-mute relative overflow-hidden"
 style={{ minHeight: 340, borderLeft: flip ? 'none' : 'var(--hairline)', borderRight: flip ? 'var(--hairline)' : 'none' }}>
      <span className="kanji text-accent" style={{ fontSize: 220, lineHeight: 1, opacity: 0.92 }}>{c.kanji}</span>
      <span className="mono absolute text-ink-faint" style={{ bottom: 18, right: 20, fontSize: 11, letterSpacing: '0.14em' }}>{c.gloss}</span>
    </div>
  );
  const body = (
    <div className="p-12">
      <div className="mb-6 flex items-center justify-between">
        <span className="mono text-ink-faint" style={{ fontSize: 13 }}>{c.id === 'torii' ? '01' : '02'}</span>
        <span className="zs-badge zs-badge-warning" style={{ whiteSpace: 'nowrap' }}>{c.status}</span>
      </div>
      <div className="gap-3 mb-2 flex items-baseline flex-wrap">
        <span className="display text-ink font-normal" style={{ fontSize: 28, letterSpacing: '-0.02em' }}>{c.name}</span>
        <span className="zs-eyebrow">{c.category}</span>
      </div>
      <p className="display text-ink m-0 font-light" style={{ fontSize: 22, letterSpacing: '-0.015em', lineHeight: 1.3 }}>{c.tagline}</p>
      <p className="text-ink-soft mt-3" style={{ fontSize: 15, lineHeight: 1.65, maxWidth: 460 }}>{c.blurb}</p>
      <div className="gap-2 mt-6 mb-8 flex flex-wrap">
        {c.surfaces.map((s) => (
          <span key={s} className="text-ink-soft text-sm border border-paper-edge rounded-full whitespace-nowrap"
 style={{ padding: '4px 12px' }}>{s}</span>
        ))}
      </div>
      <MetaChips meta={c.meta} />
    </div>
  );
  return (
    <div id={c.id} className="zs-card-flush grid overflow-hidden"
 style={{ gridTemplateColumns: flip ? '1fr 1.3fr' : '1.3fr 1fr', '--accent': `var(--acc-${c.id})` }}>
      {flip ? panel : body}
      {flip ? body : panel}
    </div>
  );
}

function Clients() {
  return (
    <section id="clients" className="sec border-t border-paper-edge">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12">
        <SectionHead kanji="二" eyebrow="Ni · two" title="Two tools, one temperament."
          sub="Torii is what your team opens in the morning. Seiki is where you go when you need to know why — and to change it." />
        <div className="gap-8 flex flex-col">
          {CLIENTS.map((c, i) => <ClientCard key={c.id} c={c} flip={i % 2 === 1} />)}
        </div>
      </div>
    </section>
  );
}

// ─── Capabilities ────────────────────────────────────────────────────
function Capabilities() {
  return (
    <section id="capabilities" className="sec border-t border-paper-edge">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12">
        <SectionHead kanji="具" eyebrow="Gu · the instruments" title="What the pair is made of." />
        <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-x-12 gap-y-12 grid">
          {CAPABILITIES.map((c) => (
            <div key={c.title}>
              <span className="kanji text-accent block" style={{ fontSize: 32, lineHeight: 1, marginBottom: 16 }}>{c.kanji}</span>
              <h3 className="display text-ink m-0 font-normal" style={{ fontSize: 17, letterSpacing: '-0.01em' }}>{c.title}</h3>
              <p className="text-ink-soft mt-2 m-0" style={{ fontSize: 13, lineHeight: 1.65 }}>{c.text}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── Privacy ─────────────────────────────────────────────────────────
function Privacy() {
  return (
    <section id="privacy" className="sec border-t border-paper-edge">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12">
        <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-16 grid items-start">
          <div>
            <SectionHead kanji="蔵" eyebrow="Zō · to keep" title="Your prompts stay in your building." />
            <p className="text-ink-soft m-0" style={{ fontSize: 15, lineHeight: 1.7, maxWidth: 460 }}>
              Both clients run on your own infrastructure. There is no Sensei HQ account, no
              telemetry back to us, and no copy of your traffic anywhere we can read it.
              You hold the provider keys; you choose the region; you can export or delete
              the whole audit log whenever you like.
            </p>
          </div>
          <div className="zs-card">
            <Eyebrow style={{ marginBottom: 16 }}>What leaves your network</Eyebrow>
            <div className="flex flex-col">
              {[
                ['Prompts and responses', 'Only to the provider you routed them to.'],
                ['Audit log', 'Never. It lives in your database.'],
                ['Usage telemetry', 'Never. There is none to send.'],
                ['Licence check', 'Once, at install. Offline after that.'],
              ].map(([k, v], i, arr) => (
                <div key={k} className="py-3 flex items-baseline justify-between gap-6"
 style={{ borderBottom: i === arr.length - 1 ? 'none' : 'var(--hairline)' }}>
                  <span className="text-ink text-sm">{k}</span>
                  <span className="text-ink-mute text-sm" style={{ textAlign: 'right', maxWidth: 240 }}>{v}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Gateway ─────────────────────────────────────────────────────────
function Gateway() {
  return (
    <section id="gateway" className="sec border-t border-paper-edge">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12">
        <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-16 grid items-center">
          <div>
            <SectionHead kanji="基" eyebrow="Ki · the foundation" title="Both are built on Gateway." />
            <p className="text-ink-soft m-0" style={{ fontSize: 15, lineHeight: 1.7, maxWidth: 460 }}>
              Gateway is our Rust library for talking to model providers: fallback chains,
              budget control, and one interface across every vendor you use. Torii and Seiki
              are what it looks like with a face on it — but the library stands alone, and
              you can build your own client on it.
            </p>
            <a href={GATEWAY_URL} className="zs-btn zs-btn-secondary mt-6">gateway.sensei-hq.com →</a>
          </div>
          <div className="zs-card">
            <Eyebrow style={{ marginBottom: 16 }}>What Gateway handles</Eyebrow>
            <div className="flex flex-col">
              {[
                ['Fallback chains', 'Primary, secondary, local — in order, on failure.'],
                ['Budget control', 'Hard ceilings per key, per team, per period.'],
                ['One interface', 'Every provider behind a single Rust API.'],
                ['Rust, embedded', 'A library in your service, not another hop.'],
              ].map(([k, v], i, arr) => (
                <div key={k} className="py-3 flex items-baseline justify-between gap-6"
 style={{ borderBottom: i === arr.length - 1 ? 'none' : 'var(--hairline)' }}>
                  <span className="text-ink text-sm">{k}</span>
                  <span className="text-ink-mute text-sm" style={{ textAlign: 'right', maxWidth: 240 }}>{v}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Contact ─────────────────────────────────────────────────────────
function Contact() {
  return (
    <section id="contact" className="sec border-t border-paper-edge">
      <div style={{ maxWidth: 720 }} className="mx-auto px-12 text-center">
        <span className="kanji text-accent block" style={{ fontSize: 40, lineHeight: 1, marginBottom: 20 }}>入</span>
        <h2 className="display text-ink m-0 font-light" style={{ fontSize: 40, lineHeight: 1.15, letterSpacing: '-0.022em' }}>
          Torii and Seiki are in private beta.
        </h2>
        <p className="text-ink-soft mt-4 m-0" style={{ fontSize: 17, lineHeight: 1.6 }}>
          Tell us how your organization works and we will tell you honestly whether the gate fits yet.
        </p>
        <div className="gap-3 mt-8 flex items-center justify-center flex-wrap">
          <a href="mailto:hi@sensei-hq.com" className="zs-btn zs-btn-primary zs-btn-lg">
            <span className="kanji text-on-primary" style={{ fontSize: 15, lineHeight: 1 }}>文</span>
            Request access
          </a>
          <a href="Sensei HQ.html" className="zs-btn zs-btn-secondary zs-btn-lg">See the whole workshop</a>
        </div>
        <a href="mailto:hi@sensei-hq.com" className="mono text-ink-mute mt-4 inline-block whitespace-nowrap"
 style={{ fontSize: 13, letterSpacing: '0.02em' }}>hi@sensei-hq.com</a>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="border-t border-paper-edge">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12 py-16 flex items-center justify-between flex-wrap" >
        <a href="Sensei HQ.html" className="gap-3 flex items-center">
          <Enso size={24} stroke="var(--accent)" />
          <span className="gap-2 flex items-baseline">
            <span className="display text-ink" style={{ fontSize: 15 }}>Sensei</span>
            <span className="mono text-ink-mute" style={{ fontSize: 11, letterSpacing: '0.08em' }}>HQ</span>
          </span>
        </a>
        <span className="mono text-ink-faint" style={{ fontSize: 11 }}>門 · Torii · Seiki · built on Gateway</span>
      </div>
    </footer>
  );
}

// ─── Root ────────────────────────────────────────────────────────────
const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "dark": false,
  "density": "airy",
  "accentMode": "distinct"
}/*EDITMODE-END*/;

function ToriiSeikiPage() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);

  hE(() => {
    document.documentElement.setAttribute('data-theme', t.dark ? 'dark' : 'light');
  }, [t.dark]);

  const set = ACCENT[t.dark ? 'dark' : 'light'];
  const accentVars = { '--accent': set.torii };
  ['torii', 'seiki'].forEach((id) => {
    accentVars[`--acc-${id}`] = t.accentMode === 'distinct' ? set[id] : set.torii;
  });

  const secPad = t.density === 'compact' ? 56 : 92;

  return (
    <div className="sensei min-h-full bg-paper relative" style={accentVars}>
      <style>{`
        .sensei .sec{ padding-top:${secPad}px; padding-bottom:${secPad}px; }
        .sensei .display, .sensei .zs-display, .sensei .zs-h1, .sensei .zs-h2,
        .sensei .zs-h3, .sensei h1, .sensei h2, .sensei h3,
        .sensei .zs-hero, .sensei .zs-display-lg{
          font-optical-sizing:none;
          font-variation-settings:"opsz" 16;
          font-feature-settings:"ss01" 0;
        }
      `}</style>
      <Nav />
      <Hero />
      <Planes />
      <Clients />
      <Capabilities />
      <Privacy />
      <Gateway />
      <Contact />
      <Footer />

      <TweaksPanel title="Tweaks">
        <TweakSection label="Theme" />
        <TweakToggle label="Dark mode" value={t.dark} onChange={(v) => setTweak('dark', v)} />
        <TweakSection label="Layout" />
        <TweakRadio label="Density" value={t.density} options={['airy', 'compact']}
                    onChange={(v) => setTweak('density', v)} />
        <TweakSection label="Client color" />
        <TweakRadio label="Accents" value={t.accentMode}
                    options={[{ value: 'distinct', label: 'Distinct' }, { value: 'unified', label: 'Unified' }]}
                    onChange={(v) => setTweak('accentMode', v)} />
      </TweaksPanel>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<ToriiSeikiPage />);
