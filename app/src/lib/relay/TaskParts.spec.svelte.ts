// @vitest-environment jsdom
// The task view's parts (#425; COMPONENTS §3d TaskHeader, RunDetails, HandoffCard): each
// draws toTaskView's data.
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayHandoffView, RelayRunDetailsView, RelayTaskHeaderView } from '@sensei/state';
import TaskHeader from './TaskHeader.svelte';
import RunDetails from './RunDetails.svelte';
import HandoffCard from './HandoffCard.svelte';

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
function text(el: Element | null | undefined): string {
  if (!el) return '';
  const copy = el.cloneNode(true) as Element;
  copy.querySelectorAll('[aria-hidden="true"]').forEach((n) => n.remove());
  return (copy.textContent ?? '').replace(/\s+/g, ' ').trim();
}

const header = (over: Partial<RelayTaskHeaderView> = {}): RelayTaskHeaderView => ({
  back: { label: 'Now', href: '/relay' },
  title: 'Diagrams header on ScreenShell',
  state: { label: 'done', tone: 'success' },
  ticket: { provider: 'Jira', key: 'SEN-52', url: 'https://x/SEN-52' },
  facts: [
    { k: 'Lane', v: 'Diagrams · L1' },
    { k: 'Assistant', v: 'Claude Code 2.1 · claude-opus-5-5' },
    { k: 'Time', v: '06:00 → 10:02 · 4 h 2 min' },
    { k: 'Rounds', v: 'tests 1 · impl 2' },
  ],
  ...over,
});

describe('TaskHeader', () => {
  it('back to where it came from, the title in the display face with its state, the facts', () => {
    const c = mount(TaskHeader, { header: header() });
    const back = c.querySelector('a[data-back]') as HTMLAnchorElement;
    expect(back.getAttribute('href')).toBe('/relay');
    expect(text(back)).toBe('Now');
    const title = c.querySelector('[data-task-title]')!;
    expect(text(title)).toBe('Diagrams header on ScreenShell');
    expect(title.className).toMatch(/\bdisplay\b/);
    expect(c.querySelector('[data-task-state] [data-relay-pill]')?.getAttribute('data-tone')).toBe('success');
    expect([...c.querySelectorAll('[data-task-fact]')].map(text)).toEqual([
      'Lane Diagrams · L1',
      'Assistant Claude Code 2.1 · claude-opus-5-5',
      'Time 06:00 → 10:02 · 4 h 2 min',
      'Rounds tests 1 · impl 2',
    ]);
  });

  it('the ticket is a pill linking out (#381)', () => {
    const c = mount(TaskHeader, { header: header() });
    const ticket = c.querySelector('a[data-ticket]') as HTMLAnchorElement;
    expect(ticket.getAttribute('href')).toBe('https://x/SEN-52');
    expect(ticket.getAttribute('target')).toBe('_blank');
    expect(text(ticket)).toBe('Jira SEN-52');
    expect(mount(TaskHeader, { header: header({ ticket: null }) }).querySelector('[data-ticket]')).toBeNull();
  });
});

describe('RunDetails', () => {
  const run = (): RelayRunDetailsView => ({
    line: '3.1 M tokens in · 2 h 22 min working · 1 subagent',
    rows: [
      { k: 'Repository', v: 'sensei-hq/sensei' },
      { k: 'Peak memory', v: null },
    ],
    note: 'Totals over the task’s turns · 2 of 4 reported',
  });

  it('folded: the summary line; open: each row, a value nobody reported reading so; radius 12 (#388)', () => {
    const c = mount(RunDetails, { run: run() });
    const details = c.querySelector('details[data-run-details]')!;
    expect(details.className).toMatch(/rounded-xl/);
    expect(text(details.querySelector('summary'))).toBe('Run details · 3.1 M tokens in · 2 h 22 min working · 1 subagent');
    expect(text(details.querySelector('[data-run-detail="Peak memory"]'))).toBe('Peak memory not reported');
    expect(details.querySelector('[data-run-detail="Peak memory"] [data-not-reported]')).not.toBeNull();
    expect(text(details.querySelector('[data-run-note]'))).toContain('2 of 4 reported');
  });
});

describe('HandoffCard', () => {
  it('the label, the brief, the docs as chips; radius 12 (#388)', () => {
    const handoff: RelayHandoffView = { label: 'Handed off · 06:00', brief: 'Move the header onto the shell.', docs: ['coord/a.md', 'coord/b.md'] };
    const c = mount(HandoffCard, { handoff });
    const card = c.querySelector('[data-handoff]')!;
    expect(card.className).toMatch(/rounded-xl/);
    expect(text(card.querySelector('[data-handoff-label]'))).toBe('Handed off · 06:00');
    expect(text(card.querySelector('[data-handoff-brief]'))).toBe('Move the header onto the shell.');
    expect([...card.querySelectorAll('[data-handoff-doc]')].map(text)).toEqual(['coord/a.md', 'coord/b.md']);
  });
});
