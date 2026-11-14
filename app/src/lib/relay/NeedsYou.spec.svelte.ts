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
  nudge: {
    label: 'Nudge the coordinator',
    canNudge: true,
    nudgedText: null,
    message: { crew_id: 'c', to: { kind: 'coordinator' }, task_id: 't-er', text: 'x', nudge: true, reason: 'merge_pending', waiting_min: 52 },
  },
  ...over,
});
const nudged = (over: Partial<RelayWaitRowView['nudge']>) => ({ ...wait().nudge, ...over });

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

  it('Nudge {party} sends the row; an idle lane is no link (#419)', async () => {
    const onnudge = vi.fn();
    const c = mount(WaitList, {
      rows: [wait(), wait({ id: 'l6', minutes: null, minutesText: '', over: false, task: 'Nothing assigned', href: null })],
      onnudge,
    });
    const button = c.querySelector('[data-wait-row="t-er"] [data-nudge]') as HTMLButtonElement;
    expect(text(button)).toBe('Nudge the coordinator');
    button.click();
    expect(onnudge).toHaveBeenCalledWith(expect.objectContaining({ id: 't-er' }));
    expect(c.querySelector('[data-wait-row="l6"] a')).toBeNull();
  });

  it('once you nudged, the button gives way to when; an automatic nudge shows beside the button', () => {
    const mine = mount(WaitList, { rows: [wait({ nudge: nudged({ canNudge: false, nudgedText: 'Nudged just now · by you' }) })], onnudge: vi.fn() });
    expect(mine.querySelector('[data-nudge]')).toBeNull();
    expect(text(mine.querySelector('[data-nudged]'))).toBe('Nudged just now · by you');
    const auto = mount(WaitList, { rows: [wait({ nudge: nudged({ nudgedText: 'Sensei nudged at 10:23' }) })], onnudge: vi.fn() });
    expect(auto.querySelector('[data-nudge]')).not.toBeNull();
    expect(text(auto.querySelector('[data-nudged]'))).toBe('Sensei nudged at 10:23');
  });

  it('a nudge in flight is busy and cannot be sent twice; a refusal says why on its row', () => {
    const busy = mount(WaitList, { rows: [wait()], onnudge: vi.fn(), nudging: ['t-er'] });
    const button = busy.querySelector('[data-nudge]') as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(button.getAttribute('aria-busy')).toBe('true');
    const refused = mount(WaitList, { rows: [wait()], onnudge: vi.fn(), errors: { 't-er': 'that lane has gone' } });
    expect(text(refused.querySelector('[data-nudge-error]'))).toBe('that lane has gone');
  });
});
