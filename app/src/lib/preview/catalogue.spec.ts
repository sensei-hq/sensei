import { describe, it, expect } from 'vitest';
import { componentsOf, entriesFor, previewHref, readView, type PreviewEntry } from './catalogue.js';

const e = (id: string): PreviewEntry => ({ id, note: `${id} note`, component: (() => null) as never, props: {} });
const CATALOGUE = [e('LaneList/mockup'), e('LaneList/phone'), e('WaitList/waiting'), e('Pill/neutral'), e('WaitList/nudged-by-you')];

describe('the preview catalogue', () => {
  it('lists each component once, in catalogue order, with how many fixtures it has', () => {
    expect(componentsOf(CATALOGUE)).toEqual([
      { name: 'LaneList', count: 2 },
      { name: 'WaitList', count: 2 },
      { name: 'Pill', count: 1 },
    ]);
  });

  it('a component’s fixtures, in catalogue order; all of them for `all`; none for an unknown name', () => {
    expect(entriesFor(CATALOGUE, 'WaitList').map((x) => x.id)).toEqual(['WaitList/waiting', 'WaitList/nudged-by-you']);
    expect(entriesFor(CATALOGUE, 'all')).toHaveLength(5);
    expect(entriesFor(CATALOGUE, 'Nope')).toEqual([]);
    // A prefix is not a component: "Lane" names nothing.
    expect(entriesFor(CATALOGUE, 'Lane')).toEqual([]);
  });

  it('reads the view from the query: theme, marks, width and the fixture, unknown values falling back', () => {
    const q = (s: string) => new URL(`http://x/preview/WaitList?${s}`).searchParams;
    expect(readView(q(''))).toEqual({ theme: 'light', glyph: 'kanji', width: 'wide', fixture: null });
    expect(readView(q('theme=dark&glyph=icon&width=phone&fixture=nudged-by-you'))).toEqual({
      theme: 'dark',
      glyph: 'icon',
      width: 'phone',
      fixture: 'nudged-by-you',
    });
    expect(readView(q('theme=sepia&glyph=emoji&width=tv'))).toMatchObject({ theme: 'light', glyph: 'kanji', width: 'wide' });
  });

  it('links keep the view and change only what is asked; the default fixture drops out of the link', () => {
    const view = { theme: 'dark', glyph: 'icon', width: 'wide', fixture: 'waiting' } as const;
    expect(previewHref('WaitList', view, { width: 'phone' })).toBe('/preview/WaitList?theme=dark&glyph=icon&width=phone&fixture=waiting');
    expect(previewHref('WaitList', view, { fixture: null })).toBe('/preview/WaitList?theme=dark&glyph=icon&width=wide');
    expect(previewHref('Pill', view, { fixture: null })).toBe('/preview/Pill?theme=dark&glyph=icon&width=wide');
  });
});
