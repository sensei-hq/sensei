// @vitest-environment jsdom
// Past runs under the Crew view (#424, the UI half of #393; CONTRACT §1i): a display-only
// rokkit List card of ended runs.
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayPastRunView } from '@sensei/state';
import PastRunsList from './PastRunsList.svelte';

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
/** What a person reads: the text, without the decorative marks. */
function text(el: Element | null | undefined): string {
  if (!el) return '';
  const copy = el.cloneNode(true) as Element;
  copy.querySelectorAll('[aria-hidden="true"]').forEach((n) => n.remove());
  return (copy.textContent ?? '').replace(/\s+/g, ' ').trim();
}

describe('PastRunsList (CONTRACT §1i; the UI half of #393)', () => {
  const past = (over: Partial<RelayPastRunView> = {}): RelayPastRunView => ({
    id: 'run-ab13',
    title: 'Run ab13',
    name: 'Sprint 11',
    state: { label: 'completed', tone: 'success' },
    span: 'Nov 1 – Nov 2',
    done: '6 of 6 tasks done',
    members: '3 members',
    deletes: { text: 'deleted in 17 days', warning: false },
    href: null,
    ...over,
  });

  it('Past runs · N, one display-only row each: id and name, span, tasks done, members, when deleted', () => {
    const c = mount(PastRunsList, { rows: [past(), past({ id: 'run-ab10', title: 'Run ab10', deletes: { text: 'deleted in 2 days', warning: true } })] });
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Past runs · 2');
    const card = c.querySelector('nav[data-list][data-variant="card"]')!;
    expect(card.querySelector('button')).toBeNull();
    const row = c.querySelector('[data-past-run="run-ab13"]')!;
    for (const [part, want] of [
      ['title', 'Run ab13'],
      ['name', 'Sprint 11'],
      ['span', 'Nov 1 – Nov 2'],
      ['done', '6 of 6 tasks done'],
      ['members', '3 members'],
      ['deletes', 'deleted in 17 days'],
    ]) {
      expect(text(row.querySelector(`[data-part="${part}"]`)), part).toBe(want);
    }
    expect(row.querySelector('[data-part="deletes"]')?.getAttribute('data-warning')).toBe('false');
    expect(c.querySelector('[data-past-run="run-ab10"] [data-part="deletes"]')?.getAttribute('data-warning')).toBe('true');
  });

  it('a row with somewhere to go is a link to the run, read-only', () => {
    const c = mount(PastRunsList, { rows: [past({ href: '/relay/runs/run-ab13' })] });
    const link = c.querySelector('[data-past-run="run-ab13"]')!.closest('[data-list-item]')!;
    expect(link.tagName).toBe('A');
    expect(link.getAttribute('href')).toBe('/relay/runs/run-ab13');
  });

  it('no past runs: nothing drawn', () => {
    const c = mount(PastRunsList, { rows: [] });
    expect(c.querySelector('[data-relay-section="past-runs"]')).toBeNull();
  });
});

