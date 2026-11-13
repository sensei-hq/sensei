import { describe, it, expect } from 'vitest';
import { toStats } from '@sensei/state';
import { toShellInput, type ShellSource } from './relay-shell-input.js';
import { WAIT_THRESHOLD_MIN } from './relay-shell.js';
import {
  coordinatorLane,
  decision,
  floatingReview,
  lane,
  minutesAgo,
  now,
  sprint,
} from '$lib/test-stubs/relay-fixtures.js';

// From what the screen holds (the read, its state, the person's view) to the
// shell's input (relay-shell.ts). Pure: the stats are counted from the read
// the daemon gave, never guessed, and a number it did not give reads as such.

const source = (over: Partial<ShellSource> = {}): ShellSource => ({
  surface: 'desktop',
  where: 'sensei',
  now: now(),
  failed: false,
  reading: false,
  view: 'now',
  composing: false,
  statFilter: null,
  queueCount: 7,
  ...over,
});

describe('toShellInput · load and run', () => {
  it('a read in flight is loading, a failed read is error, no crew is empty, else loaded', () => {
    expect(toShellInput(source({ reading: true })).load).toBe('loading');
    expect(toShellInput(source({ failed: true, now: null })).load).toBe('error');
    expect(toShellInput(source({ now: now({ crews: [], lanes: [] }) })).load).toBe('empty');
    expect(toShellInput(source()).load).toBe('loaded');
  });

  it('the run is the first open sprint, named by the end of its id; none → no run', () => {
    expect(toShellInput(source({ now: now({ runs: [sprint()] }) })).run).toEqual({ id: 'ab10' });
    expect(toShellInput(source()).run).toBeNull();
    expect(toShellInput(source({ now: now({ runs: [sprint({ state: 'completed' })] }) })).run).toBeNull();
  });

  it('the crew is the first one read, by the end of its id, and the read’s crew count travels', () => {
    expect(toShellInput(source())).toMatchObject({ crew: { id: 'c7f3' }, crews: 1 });
    expect(toShellInput(source({ now: now({ crews: [], lanes: [] }) }))).toMatchObject({ crew: null, crews: 0 });
  });

  it('passes the surface, the place, the view, the compose and filter state through', () => {
    const input = toShellInput(source({ view: 'crew', composing: true, statFilter: 'waiting' }));
    expect(input).toMatchObject({ surface: 'desktop', workspace: 'sensei', view: 'crew', from: null, compose: true, filter: 'waiting', info: true, freshness: null });
    expect(input.threshold).toBe(WAIT_THRESHOLD_MIN);
  });
});

describe('toShellInput · the main window’s narrowing (§1a)', () => {
  it('the workspace window passes no narrowing: it has no filter band', () => {
    expect(toShellInput(source()).narrow).toBeNull();
  });

  it('the main window: the read’s repositories and workspaces, and the URL’s current values', () => {
    const input = toShellInput(source({ narrow: { repo: '', workspace: 'w1' } }));
    expect(input.narrow).toEqual({
      repos: ['github.com/sensei-hq/sensei'],
      workspaces: [{ id: 'w1', name: 'Sensei' }],
      repo: '',
      workspace: 'w1',
    });
  });

  it('before a read the band keeps the URL’s values with nothing else to offer', () => {
    const input = toShellInput(source({ now: null, reading: true, narrow: { repo: 'github.com/x/y', workspace: '' } }));
    expect(input.narrow).toEqual({ repos: [], workspaces: [], repo: 'github.com/x/y', workspace: '' });
  });
});

describe('toShellInput · counts and today', () => {
  it('the Queue badge is the queue count (0 while it is unknown); the Crew badge is the lanes', () => {
    expect(toShellInput(source()).counts).toEqual({ queue: 7, crew: 2 });
    expect(toShellInput(source({ queueCount: null })).counts.queue).toBe(0);
  });

  it('Today: the completed count and the first-pass line; no review reported reads so', () => {
    expect(toShellInput(source()).today).toEqual({ done: 2, firstPass: '1 of 5 passed review first time.' });
    const quiet = now({ today: { completed: 0, reviewed: null, first_pass_approved: null } });
    expect(toShellInput(source({ now: quiet })).today).toEqual({ done: 0, firstPass: 'first pass not reported' });
    expect(toShellInput(source({ now: null, failed: true })).today).toBeNull();
  });
});

describe('toShellInput · the stats', () => {
  it('are the read counted by the relay selectors (@sensei/state toStats); no read, no stats', () => {
    expect(toShellInput(source()).stats).toEqual(toStats(now(), Date.now()));
    expect(toShellInput(source({ now: null, reading: true })).stats).toBeNull();
  });
});

describe('toShellInput · resources', () => {
  it('the build lease with its age in minutes, free disk and swap with the floor', () => {
    expect(toShellInput(source()).resources).toEqual({
      build: { resource: 'rust-build', lane: 'L1', minutes: 4 },
      disk_free_gb: 46,
      swap_free_gb: 1.2,
      swap_floor_gb: 1,
    });
  });

  it('no build lease, and numbers the daemon did not report, pass as null', () => {
    const bare = now({ resources: { build_lease: null, disk_free_gb: null, disk_floor_gb: 25, swap_free_gb: null, swap_floor_gb: 1 } });
    expect(toShellInput(source({ now: bare })).resources).toEqual({ build: null, disk_free_gb: null, swap_free_gb: null, swap_floor_gb: 1 });
    expect(toShellInput(source({ now: null, failed: true })).resources).toBeNull();
  });
});
