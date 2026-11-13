// @vitest-environment jsdom
// Needs you (#422; COMPONENTS §3b DecisionCard, WaitList/WaitRow; CONTRACT §1c-1): the
// decisions as cards, the long waits as one card of rows with Nudge, the triage line
// that opens the Queue, the unassigned tasks read-only, the lanes' messages. Each part
// draws the selectors' data and calls back; nothing here reads.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayNeedsYouView, RelayWaitRowView } from '@sensei/state';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import NeedsYou from './NeedsYou.svelte';
import WaitList from './WaitList.svelte';

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
/** What a person reads: the text, without the decorative marks (aria-hidden). */
function text(el: Element | null | undefined): string {
  if (!el) return '';
  const copy = el.cloneNode(true) as Element;
  copy.querySelectorAll('[aria-hidden="true"]').forEach((n) => n.remove());
  return (copy.textContent ?? '').replace(/\s+/g, ' ').trim();
}

const wait = (over: Partial<RelayWaitRowView> = {}): RelayWaitRowView => ({
  id: 't-er',
  minutes: 52,
  minutesText: '52 min',
  over: true,
  task: 'ER sidebar on the shell (impl r1)',
  why: 'ER diagrams · approved, not merged · waiting on the coordinator',
  href: '/relay/c/t-er',
  nudge: { label: 'Nudge the coordinator', canNudge: false },
  ...over,
});

const view = (over: Partial<RelayNeedsYouView> = {}): RelayNeedsYouView => ({
  count: 4,
  decisions: [F.decision()],
  stale: { title: '1 task has waited 15 min or more — oldest first', rows: [wait()] },
  triage: { count: 3, from: 'From Jira sprint 47, GitHub milestone 11, and an open item' },
  unassigned: null,
  messages: [],
  ...over,
});

const props = (v = view()) => ({
  view: v,
  href: (crew: string, task: string) => `/relay/${crew}/${task}`,
  onanswer: vi.fn(async () => null),
  onopenqueue: vi.fn(),
});

describe('NeedsYou', () => {
  it('the heading counts what needs the person; a decision is a card with its question, context and task', () => {
    const c = mount(NeedsYou, props());
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Needs you · 4');
    const card = c.querySelector('article[data-decision]')!;
    expect(text(card.querySelector('[data-decision-question]'))).toBe('Where does the activity log live?');
    expect(text(card)).toContain('Activity logs is waiting on this.');
    expect(card.querySelector(`a[href="/relay/${F.CREW_ID}/t-logs"]`)).not.toBeNull();
    expect([...card.querySelectorAll('[data-option][data-recommended]')].map((o) => o.getAttribute('data-option'))).toEqual(['db']);
    expect(text(card.querySelector('[data-option][data-recommended]'))).toContain('recommended');
  });

  it('the long waits: one card, titled in words, a row per wait', () => {
    const c = mount(NeedsYou, props());
    const card = c.querySelector('[data-stale-waits]')!;
    expect(text(card.querySelector('[data-stale-title]'))).toBe('1 task has waited 15 min or more — oldest first');
    expect(card.querySelectorAll('[data-wait-row]')).toHaveLength(1);
  });

  it('the triage line counts, names its sources, and opens the Queue', async () => {
    const p = props();
    const c = mount(NeedsYou, p);
    const line = c.querySelector('[data-triage-line]') as HTMLButtonElement;
    expect(text(line)).toContain('3 to triage');
    expect(text(line)).toContain('From Jira sprint 47, GitHub milestone 11, and an open item');
    line.click();
    await tick();
    expect(p.onopenqueue).toHaveBeenCalledTimes(1);
  });

  it('the tasks no lane has taken: a read-only row, not a button', () => {
    const c = mount(NeedsYou, props(view({ unassigned: { count: 4, from: 'From GitHub milestone 11 and an open item' } })));
    const row = c.querySelector('[data-unassigned]')!;
    expect(text(row)).toContain('Unassigned · 4');
    expect(text(row)).toContain('From GitHub milestone 11 and an open item');
    expect(row.querySelectorAll('button, a')).toHaveLength(0);
  });

  it('a lane’s message to the person is a row with its task', () => {
    const c = mount(NeedsYou, props(view({ messages: [F.personMessage()] })));
    expect(c.querySelector('[data-person-message]')).not.toBeNull();
  });
});

describe('WaitList · WaitRow', () => {
  it('a row: the minutes (amber when over), the task with why, a link to the task', () => {
    const c = mount(WaitList, { rows: [wait(), wait({ id: 't-x', minutes: 9, minutesText: '9 min', over: false })] });
    const [over, under] = [...c.querySelectorAll('[data-wait-row]')];
    const min = over.querySelector('[data-part="minutes"]')!;
    expect(text(min)).toBe('52 min');
    expect(min.getAttribute('data-over')).toBe('true');
    expect(min.className).toMatch(/text-warning/);
    expect(under.querySelector('[data-part="minutes"]')!.getAttribute('data-over')).toBe('false');
    expect(text(over.querySelector('[data-part="task"]'))).toBe('ER sidebar on the shell (impl r1)');
    expect(text(over.querySelector('[data-part="why"]'))).toBe('ER diagrams · approved, not merged · waiting on the coordinator');
    expect(over.querySelector('a[href="/relay/c/t-er"]')).not.toBeNull();
  });

  it('Nudge shows only when the daemon can take it (#419); an idle lane is no link', () => {
    const c = mount(WaitList, {
      rows: [wait(), wait({ id: 'l6', minutes: null, minutesText: '', over: false, task: 'Nothing assigned', href: null })],
      onnudge: vi.fn(),
    });
    expect(c.querySelector('[data-nudge]')).toBeNull();
    expect(c.querySelector('[data-wait-row="l6"] a')).toBeNull();
    const can = mount(WaitList, { rows: [wait({ nudge: { label: 'Nudge the coordinator', canNudge: true } })], onnudge: vi.fn() });
    expect(text(can.querySelector('[data-nudge]'))).toBe('Nudge the coordinator');
  });
});
