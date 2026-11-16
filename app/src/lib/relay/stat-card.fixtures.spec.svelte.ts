// The stats band's cards on the bench (/preview StatCard): one fixture per state a
// relay stat can be in, worked out by the relay's own derivation (toRelayShell), drawn
// with rokkit StatCard as the shell draws it — the kanji mark, or the icon in icon mode.
import { describe, it, expect } from 'vitest';
import { StatCard } from '@rokkit/ui';
import { propsFor, type PreviewView } from '$lib/preview/catalogue.js';
import { RELAY_CATALOGUE } from './catalogue.js';

const view = (glyph: 'kanji' | 'icon'): PreviewView => ({ theme: 'light', glyph, width: 'wide', fixture: null });
const cards = () => RELAY_CATALOGUE.filter((e) => e.id.startsWith('StatCard/'));
const card = (name: string) => {
  const found = cards().find((e) => e.id === `StatCard/${name}`);
  expect(found, `StatCard/${name}`).toBeDefined();
  return found!;
};

describe('StatCard fixtures (the relay stats band, card by card)', () => {
  it('one fixture per state a relay stat takes, in band order', () => {
    expect(cards().map((e) => e.id)).toEqual([
      'StatCard/working',
      'StatCard/working-pressed',
      'StatCard/waiting-too-long',
      'StatCard/waiting-calm',
      'StatCard/waiting-unreported',
      'StatCard/blocked',
      'StatCard/blocked-none',
      'StatCard/done',
    ]);
    for (const e of cards()) expect(e.component).toBe(StatCard);
  });

  it('each reads the relay’s derivation: label, value, tone and the line under it', () => {
    const p = (name: string) => propsFor(card(name), view('kanji'));
    expect(p('working')).toMatchObject({ label: 'Working', value: 1, tone: 'accent', description: 'on a task now' });
    expect(p('waiting-too-long')).toMatchObject({ label: 'Waiting', value: 4, tone: 'warning', description: 'oldest 52 min — too long' });
    expect(p('waiting-calm')).toMatchObject({ value: 1, tone: 'ink', description: 'oldest 4 min' });
    expect(p('waiting-unreported')).toMatchObject({ value: 2, tone: 'ink', description: 'oldest not reported' });
    expect(p('blocked')).toMatchObject({ label: 'Blocked', value: 2, tone: 'danger' });
    expect(p('blocked-none')).toMatchObject({ value: 0, tone: 'ink' });
    expect(p('done')).toMatchObject({ label: 'Completed today', value: 5, tone: 'success', description: '1 of 5 first-pass' });
  });

  it('a filter stat is a toggle; only the filtered one is pressed', () => {
    for (const e of cards()) {
      const props = propsFor(e, view('kanji'));
      expect(typeof props.onclick).toBe('function');
      expect(props.pressed).toBe(e.id === 'StatCard/working-pressed');
    }
  });

  it('the kanji mark by default; the icon in icon mode, as the shell draws it', () => {
    const kanji = propsFor(card('working'), view('kanji'));
    expect(kanji.mark).toBe('進');
    expect(kanji.icon).toBeUndefined();
    const icon = propsFor(card('working'), view('icon'));
    expect(typeof icon.icon).toBe('string');
    expect(icon.icon).not.toBe('');
  });

  it('every card keeps the description line, so a band’s cards line up', () => {
    for (const e of cards()) expect(propsFor(e, view('kanji')).reserveDescription).toBe(true);
  });
});
