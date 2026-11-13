// @vitest-environment jsdom
// QueueRow and QueueSection (#423; COMPONENTS §3c; CONTRACT §1d): a row draws the
// selector's data — title over the kind pill and the tag pills, the ticket pill (an
// external link), + in Triage, a checkbox in the backlog for a run — and calls back.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayQueueRowView } from '@sensei/state';
import QueueRow from './QueueRow.svelte';
import QueueSection from './QueueSection.svelte';

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

const triage = (over: Partial<RelayQueueRowView> = {}): RelayQueueRowView => ({
  id: 'jira:SEN-58',
  title: 'Sync shows "synced" for a refused repository',
  kind: { label: 'bug', tone: 'danger' },
  tags: ['sprint 47', 'sensei'],
  ticket: { provider: 'Jira', key: 'SEN-58', url: 'https://acme.atlassian.net/browse/SEN-58' },
  canAdd: true,
  add: { ticket_provider: 'jira', ticket_key: 'SEN-58' },
  selectable: false,
  ...over,
});

const backlog = (over: Partial<RelayQueueRowView> = {}) =>
  triage({ id: 'b1', canAdd: false, add: null, selectable: true, ...over });

describe('QueueRow', () => {
  it('the title over its pills: the kind first in its tone, then the tags', () => {
    const c = mount(QueueRow, { row: triage() });
    const row = c.querySelector('[data-queue-row="jira:SEN-58"]')!;
    expect(text(row.querySelector('[data-queue-title]'))).toBe('Sync shows "synced" for a refused repository');
    const pills = [...row.querySelectorAll('[data-queue-pills] > *')];
    expect(pills.map(text)).toEqual(['bug', 'sprint 47', 'sensei']);
    expect(pills[0].querySelector('[data-relay-pill]')?.getAttribute('data-tone')).toBe('danger');
  });

  it('the ticket is a pill linking out: provider and key, opening in a new tab', () => {
    const c = mount(QueueRow, { row: triage() });
    const ticket = c.querySelector('a[data-ticket]') as HTMLAnchorElement;
    expect(ticket.getAttribute('href')).toBe('https://acme.atlassian.net/browse/SEN-58');
    expect(ticket.getAttribute('target')).toBe('_blank');
    expect(text(ticket.querySelector('[data-ticket-provider]'))).toBe('Jira');
    expect(text(ticket.querySelector('[data-ticket-key]'))).toBe('SEN-58');
  });

  it('+ adds the row, one write per press; a backlog row has no +', async () => {
    let release: (v: string | null) => void = () => {};
    const onadd = vi.fn(() => new Promise<string | null>((r) => (release = r)));
    const c = mount(QueueRow, { row: triage(), onadd });
    const add = c.querySelector('[data-add-to-backlog]') as HTMLButtonElement;
    expect(add.getAttribute('aria-label')).toBe('Add to backlog: Sync shows "synced" for a refused repository');
    add.click();
    add.click();
    expect(onadd).toHaveBeenCalledTimes(1);
    release(null);
    await tick();
    expect(mount(QueueRow, { row: backlog() }).querySelector('[data-add-to-backlog]')).toBeNull();
  });

  it('a backlog row is picked for a run by its checkbox — under its own hook, not rokkit’s data-select', () => {
    const ontoggle = vi.fn();
    const c = mount(QueueRow, { row: backlog(), selected: true, ontoggle });
    expect(c.querySelector('[data-select]')).toBeNull();
    const box = c.querySelector('[data-relay-select]') as HTMLInputElement;
    expect(box.checked).toBe(true);
    box.click();
    expect(ontoggle).toHaveBeenCalledTimes(1);
  });

  it('no ticket, no ticket pill', () => {
    expect(mount(QueueRow, { row: triage({ ticket: null }) }).querySelector('[data-ticket]')).toBeNull();
  });
});

describe('QueueSection', () => {
  it('the heading with its mark and count, the description, one card of rows', () => {
    const c = mount(QueueSection, {
      id: 'triage',
      label: 'Triage',
      kanji: '選',
      description: 'From your active sprints in Jira, Azure DevOps and GitHub, and open items you add.',
      rows: [triage(), triage({ id: 'item-1', ticket: null })],
      onadd: vi.fn(async () => null),
    });
    const section = c.querySelector('[data-relay-section="triage"]')!;
    expect(text(section.querySelector('[data-section-heading]'))).toContain('Triage · 2');
    expect(text(section.querySelector('[data-section-description]'))).toContain('From your active sprints');
    expect(section.querySelectorAll('ul[data-queue-card] > [data-queue-row]')).toHaveLength(2);
  });
});
