// ─── Sensei HQ hub content — verbatim from docs/mockups/Sensei/hq/site.jsx ───

export interface Product {
  id: string;
  index: string;
  kanji: string;
  name: string;
  category: string;
  tagline: string;
  blurb: string;
  meta: string[];
  status: string;
  href: string;
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
  {
    id: 'sensei', index: '01', kanji: '観', name: 'Sensei',
    category: 'Desktop · Observability',
    tagline: 'A quiet companion for AI-assisted work.',
    blurb: 'Observes your sessions with AI assistants and surfaces the patterns you are too close to see. Local-first, no account, speaks only when it has something to say.',
    meta: ['macOS · Windows · Linux', 'Tauri', 'Local-first'],
    status: 'Available', href: '/sensei',
    featured: true,
    highlights: ['Watches sessions locally', 'Surfaces recurring patterns', 'Adopts memories on your terms'],
  },
  {
    id: 'dbd', index: '02', kanji: '構', name: 'DBD',
    category: 'CLI · Schema design',
    tagline: 'Schema design that lives in your terminal.',
    blurb: 'Model your database in DBML, then generate, diff and sync it across Postgres, SQLite, Convex and Supabase — all from the command line.',
    meta: ['CLI · DBML', 'Postgres · SQLite', 'Convex · Supabase'],
    status: 'Stable', href: 'https://dbd.sensei-hq.com',
  },
  {
    id: 'rokkit', index: '03', kanji: '速', name: 'Rokkit',
    category: 'Svelte · Components',
    tagline: 'Data-driven components for Svelte.',
    blurb: 'Bind a source and get a table, chart or form that just works. Headless where you need control, batteries-included where you do not.',
    meta: ['Svelte 5', 'MIT licensed', 'Open source'],
    status: 'Open source', href: 'https://rokkit.sensei-hq.com',
  },
  {
    id: 'kavach', index: '04', kanji: '守', name: 'Kavach',
    category: 'Svelte · Authentication',
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
  { name: 'sensei-hq/rokkit', accentClass: 'bg-rokkit', lang: 'Svelte', note: 'Data-driven components', href: 'https://github.com/sensei-hq/rokkit' },
  { name: 'sensei-hq/kavach', accentClass: 'bg-kavach', lang: 'TypeScript', note: 'Auth for SvelteKit', href: 'https://github.com/sensei-hq/kavach' },
];

export const NAV_LINKS = [
  ['#products', 'Products'],
  ['#incubation', 'Incubation'],
  ['#approach', 'Approach'],
  ['#open', 'Open source'],
  ['#contact', 'Contact'],
] as const;
