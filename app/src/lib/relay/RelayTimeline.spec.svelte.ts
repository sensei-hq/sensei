// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayAction, RelayTaskView } from '$lib/relay-api.js';
import RelayTimeline from './RelayTimeline.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// F8: the timeline reads thin (one line: time · actor · title) and each
// entry expands to the full detail the entry renders today.

const lanes = [
  { id: 'lane-1', code: 'L1', title: 'Coord' },
  { id: 'lane-2', code: 'L2', title: 'Worker' },
];

const action = (kind: string, over: Record<string, unknown> = {}): RelayAction =>
  ({
    id: `act-${kind}`,
    lane_code: 'L2',
    round: 1,
    phase: 'impl',
    body_md: 'Did the thing.',
    reply_to: null,
    created_at: '2026-11-06T10:11:00Z',
    kind,
    props: {},
    ...over,
  }) as unknown as RelayAction;

const smallView = (history: RelayAction[], state = 'working'): RelayTaskView =>
  ({
    task: {
      id: 'task-1',
      crew_id: 'crew-1',
      title: 'T',
      repo_key: 'github.com/o/r',
      workspace_id: null,
      phase: 'impl',
      state,
      round: 1,
      ticket: null,
      lane: null,
      assigned_at: '2026-11-06T10:00:00Z',
      completed_at: null,
      state_changed_at: '2026-11-06T10:09:00Z',
      rounds: [],
      needs_answer: false,
    },
    run: {},
    decision: null,
    handoff: null,
    history,
    lanes,
  }) as unknown as RelayTaskView;

const rows = (c: HTMLElement): HTMLElement[] =>
  [...c.querySelectorAll('[data-timeline-row]')] as HTMLElement[];

describe('RelayTimeline', () => {
  it('reads one thin line per entry: time · actor · title', () => {
    const m = mountComponent(RelayTimeline, {
      view: smallView([
        action('reported', { id: 'act-new', created_at: '2026-11-06T10:12:00Z' }),
        action('assigned', { id: 'act-old', created_at: '2026-11-06T10:11:00Z', props: { docs: [], owned_paths: [], acceptance: [] } }),
      ]),
    });
    cleanup.push(m.destroy);
    const lines = rows(m.container).map((r) => r.textContent?.replace(/\s+/g, ' ').trim());
    expect(lines).toHaveLength(2);
    expect(lines[0]).toContain('Assigned');
    expect(lines[0]).toContain('L2');
    expect(lines[1]).toContain('Reported');
    // Thin: the full detail stays folded until asked.
    expect(m.container.querySelector('[data-timeline-detail]')).toBeNull();
  });

  it('click expands the entry to its full detail, and back', () => {
    const m = mountComponent(RelayTimeline, {
      view: smallView([action('reported', { body_md: 'The full summary.' })]),
    });
    cleanup.push(m.destroy);
    const toggle = m.container.querySelector('[data-timeline-toggle]') as HTMLElement;
    expect(m.container.querySelector('[data-timeline-detail]')).toBeNull();
    toggle.click();
    flushSync();
    const detail = m.container.querySelector('[data-timeline-detail]') as HTMLElement;
    expect(detail.textContent).toContain('The full summary.');
    toggle.click();
    flushSync();
    expect(m.container.querySelector('[data-timeline-detail]')).toBeNull();
  });

  it('a cancelled task shows the derived row with honest detail', () => {
    const m = mountComponent(RelayTimeline, {
      view: smallView(
        [action('assigned', { props: { docs: [], owned_paths: [], acceptance: [] } })],
        'cancelled',
      ),
    });
    cleanup.push(m.destroy);
    expect(rows(m.container)).toHaveLength(2);
    const toggles = [...m.container.querySelectorAll('[data-timeline-toggle]')] as HTMLElement[];
    toggles[1].click();
    flushSync();
    expect(m.container.querySelector('[data-timeline-detail]')?.textContent).toContain('cancel');
  });
});
