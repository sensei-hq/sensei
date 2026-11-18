// @vitest-environment jsdom
// One history entry, to the mockup (#425; COMPONENTS §3d HistoryEntry; CONTRACT §1e.6):
// a card (radius 10) whose header row is
// the title, the outcome pill and the time; what was said in a block edged in that tone;
// a turn's files, gates and telemetry in a fold.
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { hhmm } from '$lib/relay/relay-format.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import HistoryEntry from './HistoryEntry.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});
function mount(id: string) {
  const action = F.history().find((a) => a.id === id)!;
  const m = mountComponent(HistoryEntry, { action, lanes: F.laneRefs() });
  cleanup.push(m.destroy);
  return { root: m.container.querySelector(`[data-history-entry="${id}"]`) as HTMLElement, action };
}
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

describe('HistoryEntry', () => {
  it('a card with radius 10; the timeline’s dot is TaskHistory’s, not the card’s', () => {
    const { root } = mount('act-merged');
    expect(root.className).toMatch(/rounded-\[10px\]/);
    expect(root.querySelector('[data-entry-node]')).toBeNull();
  });

  it('the header row: the title, the outcome as a pill, and the time', () => {
    const { root, action } = mount('act-review-2');
    expect(text(root.querySelector('[data-entry-title]'))).toBe('Review · impl r2');
    const outcome = root.querySelector('[data-entry-outcome]')!;
    expect(text(outcome)).toBe('approved');
    expect(outcome.querySelector('[data-pill]')?.getAttribute('data-tone')).toBe('success');
    expect(text(root.querySelector('[data-entry-time]'))).toBe(hhmm(action.created_at));
  });

  it('what was said is a block with its label, edged in the tone', () => {
    const { root } = mount('act-merged');
    const say = root.querySelector('[data-entry-say]')!;
    expect(text(say.querySelector('[data-entry-say-label]'))).toBe('Merge note');
    expect(text(say)).toContain('Merged the two reviewed commits into develop.');
    expect(say.className).toMatch(/border-l-\[3px\]/);
  });

  it('a turn’s files, gates and telemetry are folded', () => {
    const { root } = mount('act-report-2');
    const fold = root.querySelector('details[data-turn-more]') as HTMLDetailsElement;
    expect(fold).not.toBeNull();
    expect(fold.open).toBe(false);
    expect(fold.querySelectorAll('[data-turn-file]')).toHaveLength(2);
  });

  it('the by-line names the lane by codename, its code as the title (#425)', () => {
    const coded = F.history().find((a) => a.lane_code === 'L1')!;
    const { root } = mount(coded.id);
    const by = root.querySelector('[data-entry-by]')!;
    expect(text(by)).toBe('cobalt-lyra');
    expect(by.getAttribute('title')).toBe('L1');
  });

  it('a form answer shows its values, not an empty option (#389)', () => {
    const base = F.historyWithSteps().find((a) => a.id === 'act-answered')!;
    const action = { ...base, props: { option: null, values: { target: 'main' }, surface: 'desktop' } } as typeof base;
    const m = mountComponent(HistoryEntry, { action, lanes: F.laneRefs() });
    cleanup.push(m.destroy);
    const root = m.container.querySelector('[data-history-entry="act-answered"]')!;
    const line = text(root.querySelector('[data-entry-answer]'));
    expect(line).toBe('target: main · from the desktop');
  });

  it('a button answer still names its option (#389)', () => {
    const action = F.historyWithSteps().find((a) => a.id === 'act-answered')!;
    const m = mountComponent(HistoryEntry, { action, lanes: F.laneRefs() });
    cleanup.push(m.destroy);
    const root = m.container.querySelector('[data-history-entry="act-answered"]')!;
    expect(text(root.querySelector('[data-entry-answer]'))).toBe('db · from the desktop');
  });
});
