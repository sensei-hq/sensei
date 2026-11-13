// @vitest-environment jsdom
// The Now view's lists (#422; COMPONENTS §2 SectionHeading, §3b LaneList/LaneRow,
// CompletedList/CompletedRow): each draws the row data it is given (from the
// @sensei/state selectors) and nothing else — no reads, no clocks.
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayCompletedRowView, RelayLaneGroupView, RelayLaneRowView } from '@sensei/state';
import SectionHeading from './SectionHeading.svelte';
import LaneList from './LaneList.svelte';
import CompletedList from './CompletedList.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function mount<P extends Record<string, unknown>>(component: unknown, props: P) {
  const m = mountComponent(component as never, props as never);
  cleanup.push(m.destroy);
  return m.container;
}

const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
const part = (row: Element, name: string) => row.querySelector(`[data-part="${name}"]`);

const working = (over: Partial<RelayLaneRowView> = {}): RelayLaneRowView => ({
  id: 'l1',
  code: 'L1',
  role: 'contributor',
  name: 'Diagrams',
  who: 'cobalt-lyra · Claude Code 2.1',
  task: 'Structure view on the shell (tests r1)',
  state: { label: 'working', tone: 'accent' },
  when: { text: '38 min', over: false },
  href: '/relay/c/t-structure',
  ...over,
});

const group = (rows: RelayLaneRowView[], label: string | null = null): RelayLaneGroupView => ({ id: 'crew-1', label, rows });

describe('SectionHeading', () => {
  it('the label and the count, as one heading', () => {
    const c = mount(SectionHeading, { label: 'Lanes', count: 7 });
    const h = c.querySelector('h3[data-section-heading]');
    expect(text(h)).toBe('Lanes · 7');
  });

  it('no count, no dot', () => {
    const c = mount(SectionHeading, { label: 'Needs you' });
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Needs you');
  });
});

describe('LaneList · LaneRow', () => {
  it('the heading counts every lane; one card of rows, in order', () => {
    const c = mount(LaneList, { groups: [group([working(), working({ id: 'l2', code: 'L2' })])] });
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Lanes · 2');
    const card = c.querySelector('ul[data-lanes-card]');
    expect(card).not.toBeNull();
    expect([...card!.querySelectorAll(':scope > li[data-lane-row]')].map((li) => li.getAttribute('data-lane-row'))).toEqual(['L1', 'L2']);
  });

  it('a row on a task is one link with the mockup’s parts: mark, name, task, state, when, chevron', () => {
    const c = mount(LaneList, { groups: [group([working()])] });
    const li = c.querySelector('[data-lane-row="L1"]')!;
    const links = li.querySelectorAll('a');
    expect(links).toHaveLength(1);
    expect(links[0].getAttribute('href')).toBe('/relay/c/t-structure');
    expect([...links[0].children].map((el) => el.getAttribute('data-part'))).toEqual(['mark', 'name', 'task', 'state', 'when', 'chevron']);
    expect(text(part(li, 'name'))).toBe('Diagrams cobalt-lyra · Claude Code 2.1');
    expect(text(part(li, 'task'))).toBe('Structure view on the shell (tests r1)');
    expect(text(part(li, 'state'))).toBe('working');
    expect(part(li, 'state')?.getAttribute('data-tone')).toBe('accent');
    expect(text(part(li, 'when'))).toBe('38 min');
  });

  it('an idle lane is no link and has no chevron', () => {
    const c = mount(LaneList, {
      groups: [group([working({ task: 'Nothing assigned', state: { label: 'idle', tone: 'neutral' }, when: { text: '', over: false }, href: null })])],
    });
    const li = c.querySelector('[data-lane-row="L1"]')!;
    expect(li.querySelectorAll('a, button')).toHaveLength(0);
    expect(part(li, 'chevron')).toBeNull();
    expect(text(part(li, 'task'))).toBe('Nothing assigned');
  });

  it('a wait past the threshold is marked, not only coloured', () => {
    const c = mount(LaneList, { groups: [group([working({ when: { text: '52 min', over: true } })])] });
    const when = part(c.querySelector('[data-lane-row="L1"]')!, 'when')!;
    expect(when.getAttribute('data-over')).toBe('true');
    expect(when.className).toMatch(/text-warning/);
    expect(when.className).toMatch(/font-semibold/);
  });

  it('the main window: a group per crew, each under its repository', () => {
    const c = mount(LaneList, {
      groups: [
        { id: 'c1', label: 'github.com/sensei-hq/sensei', rows: [working()] },
        { id: 'c2', label: 'github.com/jerrythomas/scratch', rows: [working({ id: 'l9', code: 'L9' })] },
      ],
    });
    expect([...c.querySelectorAll('[data-crew-group]')].map((g) => g.getAttribute('data-crew-group'))).toEqual(['c1', 'c2']);
    expect([...c.querySelectorAll('[data-crew-heading]')].map(text)).toEqual(['github.com/sensei-hq/sensei', 'github.com/jerrythomas/scratch']);
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Lanes · 2');
  });
});

describe('CompletedList · CompletedRow', () => {
  const done = (over: Partial<RelayCompletedRowView> = {}): RelayCompletedRowView => ({
    id: 't-header',
    at: '10:02',
    title: 'Diagrams header on ScreenShell',
    lane: 'Diagrams · cobalt-lyra',
    outcome: { label: 'merged', tone: 'success' },
    href: '/relay/c/t-header',
    ...over,
  });

  it('the heading counts the rows; each row is one link: time, title, lane, outcome, chevron', () => {
    const c = mount(CompletedList, { rows: [done(), done({ id: 't2', outcome: { label: 'approved', tone: 'success' } })] });
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Completed today · 2');
    const row = c.querySelector('[data-completed-row="t-header"]')!;
    const links = row.querySelectorAll('a');
    expect(links).toHaveLength(1);
    expect(links[0].getAttribute('href')).toBe('/relay/c/t-header');
    expect([...links[0].children].map((el) => el.getAttribute('data-part'))).toEqual(['at', 'title', 'lane', 'outcome', 'chevron']);
    expect(text(part(row, 'lane'))).toBe('Diagrams · cobalt-lyra');
    expect(part(row, 'outcome')?.getAttribute('data-tone')).toBe('success');
    expect(c.querySelector('ul[data-completed-card]')).not.toBeNull();
  });
});
