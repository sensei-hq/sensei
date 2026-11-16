// The draft run card and Past runs on the bench (/preview): each fixture worked out by
// the crew selectors (toRunCard, toPastRuns) from the shared relay fixtures, so the
// bench shows what the screen draws.
import { describe, it, expect } from 'vitest';
import { toPastRuns, toRunCard } from '@sensei/state';
import { propsFor, type PreviewView } from '$lib/preview/catalogue.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import { RELAY_CATALOGUE } from './catalogue.js';
import RunCard from './RunCard.svelte';
import PastRunsList from './PastRunsList.svelte';

const view: PreviewView = { theme: 'light', glyph: 'kanji', width: 'wide', fixture: null };
const fixture = (id: string) => {
  const e = RELAY_CATALOGUE.find((x) => x.id === id);
  expect(e, id).toBeDefined();
  return e!;
};

describe('RunCard draft and Past runs fixtures (#424)', () => {
  it('a draft ready to start, one waiting for an answering member, one drafted before the crew', () => {
    for (const id of ['RunCard/draft', 'RunCard/draft-waiting', 'RunCard/draft-no-crew']) expect(fixture(id).component).toBe(RunCard);
    const card = (id: string) => (propsFor(fixture(id), view).card as ReturnType<typeof toRunCard>)!;
    expect(card('RunCard/draft').draft).toMatchObject({ canStart: true, note: null });
    expect(card('RunCard/draft-waiting').draft).toMatchObject({ canStart: false, note: 'Start it once a member is answering.' });
    expect(card('RunCard/draft-no-crew').draft).toMatchObject({ canStart: false, note: 'Set up your crew to start it.' });
    expect(propsFor(fixture('RunCard/draft-refused'), view).error).toBe('the crew has no live lane');
  });

  it('past runs: completed and cancelled, one due for deletion soon', () => {
    const e = fixture('PastRunsList/mockup');
    expect(e.component).toBe(PastRunsList);
    const rows = propsFor(e, view).rows as ReturnType<typeof toPastRuns>;
    expect(rows.map((r) => r.state.label)).toEqual(['completed', 'completed', 'cancelled']);
    expect(rows.some((r) => r.deletes.warning)).toBe(true);
    expect(rows).toEqual(toPastRuns(F.pastRunList(), F.PAST_AT));
  });
});
