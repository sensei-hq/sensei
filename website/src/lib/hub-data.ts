// ─── Sensei HQ hub content — verbatim from docs/mockups/Sensei/hq/site.jsx ───
//
// The whole portfolio is data: add a product or a library by adding an entry
// here (and its accent hue in rokkit.config.js + uno.config.js). `kind` splits
// the two — products are things you open, libraries are things you build on.

export type ProductKind = 'product' | 'library';

export interface Product {
  id: string;
  kind: ProductKind;
  index: string;
  kanji: string;
  name: string;
  category: string;
  tagline: string;
  blurb: string;
  meta: string[];
  status: string;
  href: string;
  /** libraries carry the language they ship in (shown in the libraries table) */
  lang?: string;
  featured?: boolean;
  highlights?: string[];
}

export interface IncubatingProduct {
  id: string;
  kanji: string;
  name: string;
  label: string;
  category: string;
  tagline: string;
  blurb: string;
}

export interface Principle {
  kanji: string;
  label: string;
  title: string;
  text: string;
}

export interface Repo {
  name: string;
  accentClass: string;
  lang: string;
  note: string;
  href: string;
}

export const PRODUCTS: Product[] = [
  // ── Products — things you open ──────────────────────────────────────────
  {
    id: 'sensei', kind: 'product', index: '01', kanji: '観', name: 'Sensei',
    category: 'Desktop · Observability',
    tagline: 'A quiet companion for AI-assisted work.',
    blurb: 'Observes your sessions with AI assistants and surfaces the patterns you are too close to see. Local-first, no account, speaks only when it has something to say.',
    meta: ['macOS · Windows · Linux', 'Tauri', 'Dōjō for teams'],
    status: 'Available', href: '/sensei',
    featured: true,
    highlights: ['Watches sessions locally', 'Surfaces recurring patterns', 'Adopts memories on your terms'],
  },
  {
    id: 'torii', kind: 'product', index: '02', kanji: '門', name: 'Torii',
    category: 'Desktop · Member workspace',
    tagline: 'The gate your team walks through.',
    blurb: 'One client for every model your organization has signed with. Ask, keep a library, try things in the playground — and always see whether the answer ran on your device or through the gate.',
    meta: ['macOS · Windows · Linux', 'Tauri', 'Works offline'],
    status: 'Beta', href: '/torii-seiki',
  },
  {
    id: 'seiki', kind: 'product', index: '03', kanji: '社', name: 'Seiki',
    category: 'Web · Governance plane',
    tagline: 'The sanctuary behind the gate.',
    blurb: 'Where the rules are kept: every request, every provider, routing and fallback chains, and budgets that cascade down your real org structure.',
    meta: ['Self-hosted', 'SSO · SCIM', 'Full audit trail'],
    status: 'Beta', href: '/torii-seiki#seiki',
  },

  // ── Libraries — things you build on ─────────────────────────────────────
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
    status: 'Open source', href: 'https://dbd.sensei-hq.com',
  },
  {
    id: 'rokkit', kind: 'library', index: '03', kanji: '速', name: 'Rokkit', lang: 'Svelte',
    category: 'Svelte components',
    tagline: 'Data-driven components for Svelte.',
    blurb: 'Bind a source and get a table, chart or form that just works. Headless where you need control, batteries-included where you do not.',
    meta: ['Svelte 5', 'MIT licensed', 'Open source'],
    status: 'Open source', href: 'https://rokkit.sensei-hq.com',
  },
  {
    id: 'kavach', kind: 'library', index: '04', kanji: '守', name: 'Kavach', lang: 'TypeScript',
    category: 'Authentication',
    tagline: 'Auth for Svelte, without the ceremony.',
    blurb: 'Sessions, providers and route guards in a few lines. Sane defaults, escape hatches everywhere, and no vendor lock-in.',
    meta: ['SvelteKit', 'OAuth · Passkeys', 'MIT licensed'],
    status: 'Open source', href: 'https://kavach.sensei-hq.com',
  },
];

export const PRINCIPLES: Principle[] = [
  { kanji: '一', label: 'Ichi · one', title: 'One thing, done well',
    text: 'Each tool has a single job and a clear edge. We would rather ship one sharp instrument than ten blunt features.' },
  { kanji: '蔵', label: 'Zō · to keep', title: 'Yours to keep',
    text: 'Local-first wherever it makes sense. Your data lives on your machine, in formats you can read, export and delete.' },
  { kanji: '静', label: 'Sei · stillness', title: 'Quiet by default',
    text: 'No nags, no dark patterns, no telemetry you did not ask for. Our tools stay out of the way until you reach for them.' },
];

export const INCUBATING: IncubatingProduct[] = [
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

export const REPOS: Repo[] = [
  { name: 'sensei-hq/gateway', accentClass: 'bg-gateway', lang: 'Rust', note: 'Fallback chains and budgets', href: 'https://github.com/sensei-hq/gateway' },
  { name: 'sensei-hq/dbd', accentClass: 'bg-dbd', lang: 'Rust', note: 'Schema design, library and CLI', href: 'https://github.com/sensei-hq/dbd' },
  { name: 'sensei-hq/rokkit', accentClass: 'bg-rokkit', lang: 'Svelte', note: 'Data-driven components', href: 'https://github.com/sensei-hq/rokkit' },
  { name: 'sensei-hq/kavach', accentClass: 'bg-kavach', lang: 'TypeScript', note: 'Auth for SvelteKit', href: 'https://github.com/sensei-hq/kavach' },
];

export const NAV_LINKS = [
  ['#products', 'Products'],
  ['#libraries', 'Libraries'],
  ['#incubation', 'Incubation'],
  ['#approach', 'Approach'],
  ['#open', 'Open source'],
  ['#contact', 'Contact'],
] as const;

/** Products (things you open) and libraries (things you build on), split by kind. */
export const productList = (): Product[] => PRODUCTS.filter((p) => p.kind === 'product');
export const libraryList = (): Product[] => PRODUCTS.filter((p) => p.kind === 'library');
/** The single featured product (the big card at the top of the portfolio). */
export const featuredProduct = (): Product | undefined => productList().find((p) => p.featured);
