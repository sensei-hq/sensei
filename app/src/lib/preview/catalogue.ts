// The development preview's catalogue rules (#429): what a fixture entry is, how the
// entries group into components, and how a preview view reads from and writes to the
// URL. Pure — the routes load the catalogue and render; these decide.

import type { Component } from 'svelte';

/** One fixture: `Component/fixture`, what it shows, and the component with its props. */
export interface PreviewEntry {
  /** `Component/fixture`, stable: the capture's folder and the mockup map's key. */
  id: string;
  /** One line on what this fixture shows, for the person reviewing. */
  note: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  component: Component<any>;
  props: Record<string, unknown>;
  /** A shell drawn in a box of this height; on a phone the header bands stack, so it gets `phoneHeight`. */
  frame?: { height: number; phoneHeight?: number };
}

/** A component in the index: its name and how many fixtures it has. */
export interface PreviewComponent {
  name: string;
  count: number;
}

/** What a preview page draws: theme, marks, width, and one fixture or (null) all of them. */
export interface PreviewView {
  theme: 'light' | 'dark';
  glyph: 'kanji' | 'icon';
  width: 'wide' | 'phone';
  fixture: string | null;
}

/** The name that shows every fixture on one page: the capture matrix shoots it. */
export const ALL = 'all';

const componentOf = (id: string): string => id.slice(0, id.indexOf('/'));
const fixtureOf = (id: string): string => id.slice(id.indexOf('/') + 1);

/** Each component once, in catalogue order, with its fixture count. */
export function componentsOf(entries: readonly PreviewEntry[]): PreviewComponent[] {
  const counts = new Map<string, number>();
  for (const e of entries) counts.set(componentOf(e.id), (counts.get(componentOf(e.id)) ?? 0) + 1);
  return [...counts].map(([name, count]) => ({ name, count }));
}

/** A component's fixtures in catalogue order; every fixture for `all`; none for an unknown name. */
export function entriesFor(entries: readonly PreviewEntry[], name: string): PreviewEntry[] {
  return name === ALL ? [...entries] : entries.filter((e) => componentOf(e.id) === name);
}

/** The fixture part of an entry's id ("WaitList/waiting" → "waiting"). */
export const fixtureName = (entry: PreviewEntry): string => fixtureOf(entry.id);

const pick = <T extends string>(value: string | null, allowed: readonly T[], fallback: T): T =>
  value !== null && (allowed as readonly string[]).includes(value) ? (value as T) : fallback;

/** The view the query asks for; an unknown value falls back to the default. */
export function readView(query: URLSearchParams): PreviewView {
  return {
    theme: pick(query.get('theme'), ['light', 'dark'], 'light'),
    glyph: pick(query.get('glyph'), ['kanji', 'icon'], 'kanji'),
    width: pick(query.get('width'), ['wide', 'phone'], 'wide'),
    fixture: query.get('fixture') || null,
  };
}

/** A link to a component's preview: the view kept, `change` applied; no fixture means all of them. */
export function previewHref(name: string, view: PreviewView, change: Partial<PreviewView> = {}): string {
  const next = { ...view, ...change };
  const q = new URLSearchParams({ theme: next.theme, glyph: next.glyph, width: next.width });
  if (next.fixture) q.set('fixture', next.fixture);
  return `/preview/${name}?${q.toString()}`;
}
