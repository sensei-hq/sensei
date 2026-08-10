// ───────────────────────────────────────────────────────────────────
// SENSEI HQ — company / studio site
// An independent studio that ships small, sharp developer tools.
// Same Zen-Sumi world as the Sensei app: washi paper, sumi ink,
// Fraunces display, kanji glyphs, hairline rules, rationed color.
// Each product carries its own accent hue (system L/C, varied hue).
// ───────────────────────────────────────────────────────────────────

const { useState: hS, useEffect: hE } = React;
const { TweaksPanel, useTweaks, TweakSection, TweakToggle, TweakRadio } = window;

// ─── Per-product accent hues (shared lightness & chroma, varied hue) ──
const ACCENTS = {
  light: {
    sensei: 'oklch(0.580 0.150 35)',
    dbd:    'oklch(0.560 0.130 255)',
    rokkit: 'oklch(0.560 0.110 162)',
    kavach: 'oklch(0.520 0.150 305)',
    torii:   'oklch(0.560 0.140 15)',
    seiki:   'oklch(0.560 0.130 255)',
    gateway: 'oklch(0.560 0.120 85)',
    magpie: 'oklch(0.560 0.110 200)',
    kata:   'oklch(0.560 0.120 145)',
    burne:  'oklch(0.580 0.140 50)',
  },
  dark: {
    sensei: 'oklch(0.700 0.150 35)',
    dbd:    'oklch(0.700 0.130 255)',
    rokkit: 'oklch(0.730 0.110 162)',
    kavach: 'oklch(0.700 0.150 305)',
    torii:   'oklch(0.700 0.140 15)',
    seiki:   'oklch(0.700 0.130 255)',
    gateway: 'oklch(0.720 0.120 85)',
    magpie: 'oklch(0.720 0.110 200)',
    kata:   'oklch(0.720 0.120 145)',
    burne:  'oklch(0.730 0.140 50)',
  },
};

// ─── Content ──────────────────────────────────────────────────────────
const PRODUCTS = [
  {
    id: 'sensei', kind: 'product', index: '01', kanji: '観', name: 'Sensei',
    category: 'Desktop · Observability',
    tagline: 'A quiet companion for AI-assisted work.',
    blurb: 'Observes your sessions with AI assistants and surfaces the patterns you are too close to see. Local-first, no account, speaks only when it has something to say.',
    meta: ['macOS · Windows · Linux', 'Tauri', 'Dōjō for teams'],
    status: 'Available', href: 'Sensei.html',
    featured: true,
    highlights: ['Watches sessions locally', 'Surfaces recurring patterns', 'Adopts memories on your terms'],
  },
  {
    id: 'torii', kind: 'product', index: '02', kanji: '門', name: 'Torii',
    category: 'Desktop · Member workspace',
    tagline: 'The gate your team walks through.',
    blurb: 'One client for every model your organization has signed with. Ask, keep a library, try things in the playground — and always see whether the answer ran on your device or through the gate.',
    meta: ['macOS · Windows · Linux', 'Tauri', 'Works offline'],
    status: 'Beta', href: 'Torii - Seiki.html',
  },
  {
    id: 'seiki', kind: 'product', index: '03', kanji: '社', name: 'Seiki',
    category: 'Web · Governance plane',
    tagline: 'The sanctuary behind the gate.',
    blurb: 'Where the rules are kept: every request, every provider, routing and fallback chains, and budgets that cascade down your real org structure.',
    meta: ['Self-hosted', 'SSO · SCIM', 'Full audit trail'],
    status: 'Beta', href: 'Torii - Seiki.html#seiki',
  },
  {
    id: 'gateway', kind: 'library', index: '01', kanji: '関', name: 'Gateway', lang: 'Rust',
    category: 'AI gateway',
    tagline: 'Fallback chains and budgets, in one crate.',
    blurb: 'Talk to every model provider through one Rust interface. Order your fallbacks, cap the spend, and embed it in your own service rather than adding another hop.',
    meta: ['Rust crate', 'MIT licensed', 'Budget control'],
    status: 'Open source', href: 'https://gateway.sensei-hq.com',
  },
  {
    id: 'dbd', kind: 'library', index: '02', kanji: '構', name: 'DBD', lang: 'Rust',
    category: 'Schema design · CLI',
    tagline: 'Schema design that lives in your terminal.',
    blurb: 'Model your database in DBML, then generate, diff and sync it across Postgres, SQLite, Convex and Supabase — all from the command line.',
    meta: ['Rust CLI · DBML', 'MIT licensed', 'Postgres · SQLite · Convex'],
    status: 'Open source', href: '#dbd',
  },
  {
    id: 'rokkit', kind: 'library', index: '03', kanji: '速', name: 'Rokkit', lang: 'Svelte',
    category: 'Svelte components',
    tagline: 'Data-driven components for Svelte.',
    blurb: 'Bind a source and get a table, chart or form that just works. Headless where you need control, batteries-included where you do not.',
    meta: ['Svelte 5', 'MIT licensed', 'Open source'],
    status: 'Open source', href: '#rokkit',
  },
  {
    id: 'kavach', kind: 'library', index: '04', kanji: '守', name: 'Kavach', lang: 'TypeScript',
    category: 'Authentication',
    tagline: 'Auth for Svelte, without the ceremony.',
    blurb: 'Sessions, providers and route guards in a few lines. Sane defaults, escape hatches everywhere, and no vendor lock-in.',
    meta: ['SvelteKit', 'OAuth · Passkeys', 'MIT licensed'],
    status: 'Open source', href: '#kavach',
  },
];

const PRINCIPLES = [
  { kanji: '一', label: 'Ichi · one', title: 'One thing, done well',
    text: 'Each tool has a single job and a clear edge. We would rather ship one sharp instrument than ten blunt features.' },
  { kanji: '蔵', label: 'Zō · to keep', title: 'Yours to keep',
    text: 'Local-first wherever it makes sense. Your data lives on your machine, in formats you can read, export and delete.' },
  { kanji: '静', label: 'Sei · stillness', title: 'Quiet by default',
    text: 'No nags, no dark patterns, no telemetry you did not ask for. Our tools stay out of the way until you reach for them.' },
];

// Earlier-stage experiments — rougher edges, same temperament.
const INCUBATING = [
  { id: 'magpie', kanji: '集', name: 'Magpie', label: 'Shū · to gather',
    category: 'Local-first · Library',
    tagline: 'Your whole library, in one nest.',
    blurb: 'Books, comics, manga and webtoons — collected, organized and read in one place. No accounts, no clouds, just your shelf.' },
  { id: 'kata', kanji: '型', name: 'Kata', label: 'Kata · form',
    category: 'Local-first · Fitness',
    tagline: 'Training that meets you where you are.',
    blurb: 'AI programs your workouts and adapts to your real progress. No cloud subscription, no rented streaks — your training stays yours.' },
  { id: 'burne', kanji: '燃', name: 'Burn-E', label: 'Nen · to burn',
    category: 'Desktop · Fabrication',
    tagline: 'A visual G-code editor for laser work.',
    blurb: 'Design, preview and tune toolpaths for laser cutting and engraving — see the burn before you commit the material.' },
];

// ═══════════════════════════════════════════════════════════════════════
// Primitives
// ═══════════════════════════════════════════════════════════════════════

// Sensei brand mark — SVG used as a CSS mask so it fills with any brand color.
function Enso({ size = 26, stroke = 'var(--accent)' }) {
  return (
    <span className="block shrink-0" aria-hidden="true"
 style={{ width: size, height: size,
 background: stroke,
 WebkitMaskImage: 'url(uploads/sensei.svg?v=3)',
 maskImage: 'url(uploads/sensei.svg?v=3)',
 WebkitMaskSize: 'contain', maskSize: 'contain',
 WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
 WebkitMaskPosition: 'center', maskPosition: 'center' }} />
  );
}

function Eyebrow({ children, style }) {
  return (
    <div className="zs-eyebrow" style={style}>{children}</div>
  );
}

const MAXW = 1120;

// ═══════════════════════════════════════════════════════════════════════
// Nav
// ═══════════════════════════════════════════════════════════════════════
function Nav() {
  const links = [
    ['#products', 'Products'],
    ['#libraries', 'Libraries'],
    ['#incubation', 'Incubation'],
    ['#approach', 'Approach'],
    ['#open', 'Open source'],
    ['#contact', 'Contact'],
  ];
  return (
    <div className="sticky" style={{ top: 0, zIndex: 50,
 background: 'color-mix(in oklch, var(--paper) 80%, transparent)',
 backdropFilter: 'blur(14px) saturate(150%)',
 WebkitBackdropFilter: 'blur(14px) saturate(150%)',
 WebkitMaskImage: 'linear-gradient(to bottom, #000 72%, transparent)',
 maskImage: 'linear-gradient(to bottom, #000 72%, transparent)',
 paddingBottom: 6 }}>
      <nav style={{ maxWidth: MAXW }}
 className="mx-auto px-12 py-4 flex items-center justify-between">
        <a href="#top" className="gap-3 flex items-center">
          <Enso size={26} stroke="var(--accent)" />
          <span className="gap-2 flex items-baseline">
            <span className="display text-ink" style={{ fontSize: 18, letterSpacing: '-0.01em' }}>Sensei</span>
            <span className="mono text-ink-mute" style={{ fontSize: 11, letterSpacing: '0.08em' }}>HQ</span>
          </span>
        </a>
        <div className="gap-8 flex items-center">
          {links.map(([href, label]) => (
            <a key={href} href={href} className="text-ink-soft text-sm"
               style={{ transition: 'color .15s' }}
               onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
               onMouseLeave={(e) => e.currentTarget.style.color = 'var(--ink-soft)'}>
              {label}
            </a>
          ))}
          <a href="#open" className="zs-btn zs-btn-secondary zs-btn-sm" style={{ marginLeft: 4 }}>
            GitHub
          </a>
        </div>
      </nav>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Hero — studio statement + "in the workshop" preview
// ═══════════════════════════════════════════════════════════════════════
function Hero() {
  return (
    <header id="top" style={{ maxWidth: MAXW }} className="mx-auto px-12 pt-12 pb-24">
      <div style={{ gridTemplateColumns: '1.55fr 1fr' }} className="gap-16 grid items-start">
        {/* Statement */}
        <div>
          <div className="gap-3 mb-6 flex items-baseline">
            <span className="kanji text-accent" style={{ fontSize: 44, lineHeight: 1 }}>道</span>
            <Eyebrow>Dō · the way — an independent studio</Eyebrow>
          </div>
          <h1 className="display text-ink m-0 font-light"
 style={{ fontSize: 60, lineHeight: 1.08,
 letterSpacing: '-0.025em', maxWidth: 640 }}>
            We build quiet, sharp tools for the people who build software.
          </h1>
          <p className="text-ink-soft mt-6" style={{ fontSize: 17, lineHeight: 1.6, maxWidth: 540 }}>
            Sensei HQ is a small workshop of developer tools — three products you
            open, four libraries you build on. Restraint over noise, craft over
            scale, and a deep respect for the person on the other side of the screen.
          </p>
          <div 
 className="gap-3 mt-8 flex items-center flex-wrap">
            <a href="#products" className="zs-btn zs-btn-primary zs-btn-lg">
              <span className="kanji text-on-primary" style={{ fontSize: 15, lineHeight: 1 }}>見</span>
              See the tools
            </a>
            <a href="#approach" className="text-ink-soft text-sm">How we work ↓</a>
          </div>
          <div className="gap-4 mt-12 flex flex-wrap">
            {['Independent studio', 'Est. 2024', 'Three products · four libraries'].map((m) => (
              <span key={m} className="mono text-ink-mute" style={{ fontSize: 11 }}>{m}</span>
            ))}
          </div>
        </div>

        {/* In the workshop */}
        <aside className="border border-paper-edge rounded-lg bg-paper-soft overflow-hidden"
 >
          <div className="px-6 py-4 border-b border-paper-edge flex items-center justify-between"
 >
            <Eyebrow>In the workshop</Eyebrow>
            <span className="mono text-ink-faint" style={{ fontSize: 11 }}>07</span>
          </div>
          <div className="divide-y">
            {PRODUCTS.map((p) => (
              <a key={p.id} href={p.href}
 style={{ gridTemplateColumns: 'auto 1fr auto', '--accent': `var(--acc-${p.id})`,
 transition: 'background .15s' }}
 className="gap-4 px-6 py-4 grid items-center"
 onMouseEnter={(e) => e.currentTarget.style.background = 'var(--paper-mute)'}
 onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                <span className="kanji text-accent text-center" style={{ fontSize: 26, lineHeight: 1, width: 30 }}>{p.kanji}</span>
                <span>
                  <span className="display text-ink block" style={{ fontSize: 15 }}>{p.name}</span>
                  <span className="text-ink-mute text-xs">{p.category}</span>
                </span>
                <span className="zs-dot bg-accent" />
              </a>
            ))}
          </div>
        </aside>
      </div>
    </header>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Portfolio
// ═══════════════════════════════════════════════════════════════════════
function StatusBadge({ status }) {
  const cls = status === 'Available' ? 'zs-badge zs-badge-accent'
            : status === 'Beta' ? 'zs-badge zs-badge-warning'
            : 'zs-badge zs-badge-success';
  return <span className={cls} style={{ whiteSpace: 'nowrap' }}>{status}</span>;
}

function MetaChips({ meta }) {
  return (
    <div className="gap-2 flex flex-wrap">
      {meta.map((m) => (
        <span key={m} className="mono text-ink-mute border border-paper-edge rounded-sm whitespace-nowrap"
 style={{ fontSize: 10.5, padding: '2px 7px' }}>{m}</span>
      ))}
    </div>
  );
}

function FeaturedCard({ p }) {
  return (
    <a href={p.href}
 style={{ gridTemplateColumns: '1.3fr 1fr',
 '--accent': `var(--acc-${p.id})`,
 transition: 'border-color .18s, transform .18s' }}
 className="zs-card-flush grid overflow-hidden"
 onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'var(--accent)'; }}
 onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'var(--paper-edge)'; }}>
      <div className="p-12">
        <div className="mb-6 flex items-center justify-between">
          <span className="mono text-ink-faint" style={{ fontSize: 12 }}>{p.index}</span>
          <StatusBadge status={p.status} />
        </div>
        <div className="gap-3 mb-2 flex items-baseline">
          <span className="display text-ink font-normal" style={{ fontSize: 32, letterSpacing: '-0.02em' }}>{p.name}</span>
          <span className="zs-eyebrow">{p.category}</span>
        </div>
        <p className="display text-ink m-0 font-light" style={{ fontSize: 22, letterSpacing: '-0.015em', lineHeight: 1.3 }}>
          {p.tagline}
        </p>
        <p className="text-ink-soft mt-3" style={{ fontSize: 14, lineHeight: 1.65, maxWidth: 460 }}>{p.blurb}</p>
        <div className="gap-2 mt-6 mb-8 flex flex-col">
          {p.highlights.map((h) => (
            <div key={h} className="gap-3 flex items-center">
              <span className="zs-dot bg-accent" />
              <span className="text-ink-soft text-sm">{h}</span>
            </div>
          ))}
        </div>
        <div className="gap-4 flex items-center justify-between flex-wrap">
          <MetaChips meta={p.meta} />
          <span className="text-accent text-sm font-medium" >Explore Sensei →</span>
        </div>
      </div>
      {/* Kanji panel */}
      <div className="flex items-center justify-center bg-paper-mute border-l relative overflow-hidden" style={{ minHeight: 320 }}>
        <span className="kanji text-accent" style={{ fontSize: 220, lineHeight: 1,
 opacity: 0.92 }}>{p.kanji}</span>
        <span className="mono absolute uppercase text-ink-faint" style={{ bottom: 18, right: 20,
 fontSize: 10.5, letterSpacing: '0.18em' }}>Kan · to observe</span>
      </div>
    </a>
  );
}

function ProductCard({ p }) {
  return (
    <a href={p.href}
 style={{
 '--accent': `var(--acc-${p.id})`,
 transition: 'border-color .18s, transform .18s' }}
 className="zs-card p-0 flex flex-col"
 onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'var(--accent)'; e.currentTarget.style.transform = 'translateY(-2px)'; }}
 onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'var(--paper-edge)'; e.currentTarget.style.transform = 'translateY(0)'; }}>
      <div className="p-8 flex flex-col flex-1" >
        <div className="flex items-center justify-between" >
          <span className="mono text-ink-faint" style={{ fontSize: 12 }}>{p.index}</span>
          <StatusBadge status={p.status} />
        </div>
        <span className="kanji text-accent" style={{ fontSize: 56, lineHeight: 1, margin: '20px 0 16px' }}>{p.kanji}</span>
        <div className="gap-2 mb-1 flex items-baseline">
          <span className="display text-ink font-normal" style={{ fontSize: 22, letterSpacing: '-0.02em' }}>{p.name}</span>
        </div>
        <span className="zs-eyebrow mb-3">{p.category}</span>
        <p className="display text-ink m-0 font-normal" style={{ fontSize: 17, lineHeight: 1.3 }}>{p.tagline}</p>
        <p className="text-ink-soft mt-2" style={{ fontSize: 13.5, lineHeight: 1.6 }}>{p.blurb}</p>
        <div className="flex-1" />
        <div className="mt-6 mb-4"><MetaChips meta={p.meta} /></div>
        <div className="border-t border-paper-edge pt-4 flex items-center justify-between"
 >
          <span className="text-accent text-sm font-medium" >Explore {p.name} →</span>
        </div>
      </div>
    </a>
  );
}

function LibraryRow({ p }) {
  const external = p.href.startsWith('http');
  return (
    <a href={p.href}
 style={{ gridTemplateColumns: 'auto 150px 1fr auto auto', '--accent': `var(--acc-${p.id})`,
 borderTop: 'var(--hairline)', transition: 'background .15s' }}
 className="gap-6 px-6 py-5 grid items-center"
 onMouseEnter={(e) => e.currentTarget.style.background = 'var(--paper-mute)'}
 onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
      <span className="kanji text-accent text-center" style={{ fontSize: 28, lineHeight: 1, width: 34 }}>{p.kanji}</span>
      <span>
        <span className="display text-ink block" style={{ fontSize: 17, letterSpacing: '-0.01em' }}>{p.name}</span>
        <span className="mono text-ink-faint" style={{ fontSize: 11 }}>{p.lang}</span>
      </span>
      <span>
        <span className="text-ink block text-sm">{p.tagline}</span>
        <span className="text-ink-mute text-xs">{p.category}</span>
      </span>
      <StatusBadge status={p.status} />
      <span className="text-accent text-sm">{external ? '↗' : '→'}</span>
    </a>
  );
}

function Portfolio() {
  const products = PRODUCTS.filter((p) => p.kind === 'product');
  const libraries = PRODUCTS.filter((p) => p.kind === 'library');
  const featured = products.find((p) => p.featured);
  const rest = products.filter((p) => !p.featured);
  return (
    <section id="products" className="bg-paper-soft">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12 sec">
        <div className="gap-4 mb-12 flex items-end justify-between flex-wrap">
          <div>
            <Eyebrow style={{ marginBottom: 12 }}>The portfolio</Eyebrow>
            <h2 className="display text-ink m-0 font-light" style={{ fontSize: 40, letterSpacing: '-0.02em' }}>
              Three products. Four libraries.
            </h2>
          </div>
          <p className="text-ink-soft" style={{ fontSize: 14, lineHeight: 1.6, maxWidth: 360 }}>
            The products are things you open. The libraries are things you build
            on. Both carry the same temperament and the same refusal to get in
            your way.
          </p>
        </div>

        <div className="mb-6"><FeaturedCard p={featured} /></div>

        <div style={{ gridTemplateColumns: 'repeat(2, 1fr)' }} className="gap-6 grid">
          {rest.map((p) => <ProductCard key={p.id} p={p} />)}
        </div>

        <div id="libraries" className="mt-24">
          <div className="gap-4 mb-8 flex items-end justify-between flex-wrap">
            <div>
              <div className="gap-3 mb-3 flex items-baseline">
                <span className="kanji text-accent" style={{ fontSize: 28, lineHeight: 1 }}>礎</span>
                <Eyebrow>Soseki · the foundation stones</Eyebrow>
              </div>
              <h2 className="display text-ink m-0 font-light" style={{ fontSize: 28, letterSpacing: '-0.02em' }}>
                What our products stand on.
              </h2>
            </div>
            <p className="text-ink-soft" style={{ fontSize: 14, lineHeight: 1.6, maxWidth: 340 }}>
              Written for our own tools first, then published — so the thing you
              install is the thing we depend on.
            </p>
          </div>
          <div className="border-b border-paper-edge rounded-lg overflow-hidden bg-paper">
            {libraries.map((p) => <LibraryRow key={p.id} p={p} />)}
          </div>
        </div>
      </div>
    </section>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Incubation — earlier-stage experiments (dashed, distinct from portfolio)
// ═══════════════════════════════════════════════════════════════════════
function IncubationCard({ p }) {
  return (
    <div style={{
 '--accent': `var(--acc-${p.id})`,
 transition: 'border-color .18s, background .18s' }}
 className="zs-card border-dashed p-0 flex flex-col flex-1 bg-transparent"
 onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'var(--accent)'; e.currentTarget.style.background = 'var(--paper-soft)'; }}
 onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'var(--paper-edge)'; e.currentTarget.style.background = 'transparent'; }}>
      <div className="p-8 flex flex-col flex-1" >
        <div className="flex items-center justify-between" >
          <span className="kanji text-accent" style={{ fontSize: 44, lineHeight: 1 }}>{p.kanji}</span>
          <span className="zs-badge whitespace-nowrap" >
            <span className="zs-dot bg-accent" style={{ width: 6, height: 6 }} />
            Incubating
          </span>
        </div>
        <div className="gap-2 mt-6 mb-1 flex items-baseline">
          <span className="display text-ink font-normal" style={{ fontSize: 20, letterSpacing: '-0.02em' }}>{p.name}</span>
          <span className="mono text-ink-faint whitespace-nowrap" style={{ fontSize: 10.5 }}>{p.label}</span>
        </div>
        <span className="zs-eyebrow mb-3">{p.category}</span>
        <p className="display text-ink m-0 font-normal" style={{ fontSize: 16, lineHeight: 1.3 }}>{p.tagline}</p>
        <p className="text-ink-soft mt-2" style={{ fontSize: 13.5, lineHeight: 1.6 }}>{p.blurb}</p>
      </div>
    </div>
  );
}

function Incubation() {
  return (
    <section id="incubation" style={{ maxWidth: MAXW }} className="mx-auto px-12 sec">
      <div className="gap-4 mb-12 flex items-end justify-between flex-wrap">
        <div>
          <div className="gap-3 mb-3 flex items-baseline">
            <span className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>育</span>
            <Eyebrow>Iku · to nurture — in incubation</Eyebrow>
          </div>
          <h2 className="display text-ink m-0 font-light" style={{ fontSize: 40, letterSpacing: '-0.02em' }}>
            Still taking shape.
          </h2>
        </div>
        <p className="text-ink-soft" style={{ fontSize: 14, lineHeight: 1.6, maxWidth: 380 }}>
          Experiments we are nurturing in the workshop. Rougher edges, same
          temperament — these may grow into the next tools we ship.
        </p>
      </div>
      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-6 grid">
        {INCUBATING.map((p) => <IncubationCard key={p.id} p={p} />)}
      </div>
    </section>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Approach
// ═══════════════════════════════════════════════════════════════════════
function Approach() {
  return (
    <section id="approach" className="bg-paper-soft">
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12 sec">
        <div style={{ gridTemplateColumns: '1fr 1.5fr' }} className="gap-16 grid items-start">
          <div>
            <Eyebrow style={{ marginBottom: 12 }}>The approach</Eyebrow>
            <h2 className="display text-ink m-0 font-normal" style={{ fontSize: 32, letterSpacing: '-0.015em', lineHeight: 1.25 }}>
              The master observes for a long time before teaching.
            </h2>
            <p className="text-ink-soft mt-4" style={{ fontSize: 14, lineHeight: 1.7, maxWidth: 360 }}>
              The kanji on each tool name a phase of practice — observe, build,
              hasten, guard. They are what we ask of the people who use our
              tools, and what we ask of ourselves while making them.
            </p>
          </div>
          <div className="gap-0 flex flex-col">
            {PRINCIPLES.map((pr, i) => (
              <div key={pr.title}
 style={{ gridTemplateColumns: 'auto 1fr',
 borderTop: i === 0 ? 'none' : 'var(--hairline)' }}
 className="gap-6 py-6 grid items-start">
                <div style={{ width: 56 }}>
                  <span className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>{pr.kanji}</span>
                </div>
                <div>
                  <div className="gap-3 mb-1 flex items-baseline">
                    <span className="display text-ink" style={{ fontSize: 18 }}>{pr.title}</span>
                    <span className="mono text-ink-faint" style={{ fontSize: 11 }}>{pr.label}</span>
                  </div>
                  <p className="text-ink-soft m-0" style={{ fontSize: 14, lineHeight: 1.65 }}>{pr.text}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Open source band
// ═══════════════════════════════════════════════════════════════════════
function OpenSource() {
  const repos = [
    { name: 'sensei-hq/gateway', dot: 'var(--acc-gateway)', lang: 'Rust', note: 'Fallback chains and budgets' },
    { name: 'sensei-hq/dbd', dot: 'var(--acc-dbd)', lang: 'Rust', note: 'Schema design, library and CLI' },
    { name: 'sensei-hq/rokkit', dot: 'var(--acc-rokkit)', lang: 'Svelte', note: 'Data-driven components' },
    { name: 'sensei-hq/kavach', dot: 'var(--acc-kavach)', lang: 'TypeScript', note: 'Auth for SvelteKit' },
  ];
  return (
    <section id="open" style={{ maxWidth: MAXW }} className="mx-auto px-12 sec">
      <div style={{ gridTemplateColumns: '1fr 1.2fr' }} className="gap-16 grid items-center">
        <div>
          <div className="gap-3 mb-4 flex items-baseline">
            <span className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>公</span>
            <Eyebrow>Kō · in the open</Eyebrow>
          </div>
          <h2 className="display text-ink m-0 font-normal" style={{ fontSize: 28, letterSpacing: '-0.015em', lineHeight: 1.3 }}>
            All four libraries are built in the open.
          </h2>
          <p className="text-ink-soft mt-4" style={{ fontSize: 14, lineHeight: 1.7, maxWidth: 420 }}>
            Gateway, DBD, Rokkit and Kavach are MIT-licensed and developed in public.
            Read the source, file an issue, or send a pull request — the workshop door is open.
          </p>
          <a href="#github" className="zs-btn zs-btn-secondary mt-6">
            <span className="kanji text-ink-soft" style={{ fontSize: 14, lineHeight: 1 }}>叉</span>
            Browse the repositories
          </a>
        </div>
        <div className="border border-paper-edge rounded-lg bg-paper overflow-hidden" >
          <div className="divide-y">
            {repos.map((r) => (
              <a key={r.name} href="#github"
 style={{ gridTemplateColumns: 'auto 1fr auto' }}
 className="gap-4 px-6 py-4 grid items-center">
                <span className="zs-dot" style={{ background: r.dot }} />
                <span>
                  <span className="mono text-ink block" style={{ fontSize: 13 }}>{r.name}</span>
                  <span className="text-ink-mute text-xs">{r.note}</span>
                </span>
                <span className="mono text-ink-mute" style={{ fontSize: 11 }}>{r.lang}</span>
              </a>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Contact + Footer
// ═══════════════════════════════════════════════════════════════════════
function Contact() {
  return (
    <section id="contact" className="bg-paper-soft">
      <div style={{ maxWidth: 720 }} className="mx-auto px-12 sec text-center">
        <div className="mb-6 flex justify-center">
          <Enso size={40} stroke="var(--accent)" width={2.4} />
        </div>
        <h2 className="display text-ink m-0 font-light" style={{ fontSize: 32, letterSpacing: '-0.02em', lineHeight: 1.2 }}>
          Building something quiet and sharp?
        </h2>
        <p className="text-ink-soft mt-4" style={{ fontSize: 15, lineHeight: 1.7 }}>
          We like talking to people who care about the craft. Tell us what you
          are working on, or follow along as the workshop grows.
        </p>
        <div 
 className="gap-3 mt-8 flex items-center justify-center flex-wrap">
          <a href="mailto:hi@sensei-hq.com" className="zs-btn zs-btn-primary zs-btn-lg">
            <span className="kanji text-on-primary" style={{ fontSize: 15, lineHeight: 1 }}>文</span>
            Leave a note
          </a>
          <a href="#newsletter" className="zs-btn zs-btn-secondary zs-btn-lg">Join the newsletter</a>
        </div>
        <a href="mailto:hi@sensei-hq.com" className="mono text-ink-mute mt-4 inline-block whitespace-nowrap"
 style={{ fontSize: 12, letterSpacing: '0.02em' }}>
          hi@sensei-hq.com
        </a>
      </div>
    </section>
  );
}

function Footer() {
  const cols = [
    ['Products', [['Sensei', 'Sensei.html'], ['Torii', 'Torii - Seiki.html'], ['Seiki', 'Torii - Seiki.html#seiki']]],
    ['Libraries', [['Gateway', 'https://gateway.sensei-hq.com'], ['DBD', '#dbd'], ['Rokkit', '#rokkit'], ['Kavach', '#kavach']]],
    ['Studio', [['Approach', '#approach'], ['Open source', '#open'], ['Contact', '#contact']]],
    ['Connect', [['GitHub', '#github'], ['Twitter', '#twitter'], ['hi@sensei-hq.com', 'mailto:hi@sensei-hq.com']]],
  ];
  return (
    <footer>
      <div style={{ maxWidth: MAXW }} className="mx-auto px-12 py-16">
        <div style={{ gridTemplateColumns: '1.4fr 1fr 1fr 1fr 1fr' }} className="gap-10 grid items-start">
          <div>
            <div className="gap-3 mb-3 flex items-center">
              <Enso size={24} stroke="var(--accent)" />
              <span className="gap-2 flex items-baseline">
                <span className="display text-ink" style={{ fontSize: 16 }}>Sensei</span>
                <span className="mono text-ink-mute" style={{ fontSize: 11, letterSpacing: '0.08em' }}>HQ</span>
              </span>
            </div>
            <p className="text-ink-mute m-0" style={{ fontSize: 13, lineHeight: 1.6, maxWidth: 280 }}>
              A small workshop of developer tools. Built quietly, in Tennessee.
            </p>
          </div>
          {cols.map(([title, items]) => (
            <div key={title}>
              <Eyebrow style={{ marginBottom: 14 }}>{title}</Eyebrow>
              <div className="gap-2 flex flex-col">
                {items.map(([label, href]) => (
                  <a key={label} href={href} className="text-ink-soft text-sm"
                     style={{ transition: 'color .15s' }}
                     onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
                     onMouseLeave={(e) => e.currentTarget.style.color = 'var(--ink-soft)'}>{label}</a>
                ))}
              </div>
            </div>
          ))}
        </div>
        <div className="border-t border-paper-edge mt-12 pt-6 flex items-center justify-between flex-wrap"
 style={{ gap: 12 }}>
          <span className="mono text-ink-faint" style={{ fontSize: 11 }}>© 2026 Sensei HQ · All rights reserved</span>
          <span className="mono text-ink-faint" style={{ fontSize: 11 }}>道 · the way</span>
        </div>
      </div>
    </footer>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Root
// ═══════════════════════════════════════════════════════════════════════
const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "dark": false,
  "density": "airy",
  "accentMode": "distinct"
}/*EDITMODE-END*/;

function HQSite() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);

  hE(() => {
    document.documentElement.setAttribute('data-theme', t.dark ? 'dark' : 'light');
  }, [t.dark]);

  // Per-product accent custom properties, resolved against theme + mode.
  const accentVars = {};
  const set = ACCENTS[t.dark ? 'dark' : 'light'];
  [...PRODUCTS, ...INCUBATING].forEach((p) => {
    accentVars[`--acc-${p.id}`] = t.accentMode === 'distinct' ? set[p.id] : 'var(--accent)';
  });

  const secPad = t.density === 'compact' ? 56 : 92;

  return (
    <div className="sensei min-h-full bg-paper relative" style={{ ...accentVars }}>
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
      <div className="relative" style={{ zIndex: 1 }}>
        <Nav />
        <Hero />
        <Portfolio />
        <Incubation />
        <Approach />
        <OpenSource />
        <Contact />
        <Footer />
      </div>

      <TweaksPanel title="Tweaks">
        <TweakSection label="Theme" />
        <TweakToggle label="Dark mode" value={t.dark} onChange={(v) => setTweak('dark', v)} />
        <TweakSection label="Layout" />
        <TweakRadio label="Density" value={t.density}
                    options={['airy', 'compact']}
                    onChange={(v) => setTweak('density', v)} />
        <TweakSection label="Product color" />
        <TweakRadio label="Accents" value={t.accentMode}
                    options={[{ value: 'distinct', label: 'Distinct' }, { value: 'unified', label: 'Unified' }]}
                    onChange={(v) => setTweak('accentMode', v)} />
      </TweaksPanel>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<HQSite />);
