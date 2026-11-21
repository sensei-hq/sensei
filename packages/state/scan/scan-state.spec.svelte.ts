import { describe, it, expect, afterEach, beforeEach, vi } from 'vitest';
import {
  ScanWorkspaceState,
  ScanActivityState,
  ScanState,
  commonParent,
  pollStep,
  triggerScans,
  miniSummary,
  type ScanSource,
  type ScanStreamEvent,
  type IndexQueueStatus,
  type QueueCounts,
} from './scan-state.svelte.js';
import type { ScanWorkspace, ScanWorkspaceFolder, ActivityEvent } from './types.js';
import type { StateStream } from '../source.js';
import type { ApiResult } from '../result.js';

// ── Factories ────────────────────────────────────────────────────────────────

function folder(id: string, name: string, opts: Partial<ScanWorkspaceFolder> = {}): ScanWorkspaceFolder {
  return { id, name, path: `/code/${name}`, stack: [], filesTotal: 0, filesCompleted: 0, status: 'discovered', ...opts };
}

function workspace(id: string, name: string, folders: ScanWorkspaceFolder[], opts: Partial<ScanWorkspace> = {}): ScanWorkspace {
  return { id, name, status: 'scanning', folders, autoDetected: true, confidence: 'high', ...opts };
}

function activity(id: string, level: ActivityEvent['level'], message: string, elapsed = 0): ActivityEvent {
  return { id, level, message, elapsed, timestamp: Date.now() };
}

// ── commonParent ─────────────────────────────────────────────────────────────

describe('commonParent', () => {
  it('single path returns parent dir', () => {
    expect(commonParent(['/code/lumen/lumen-app'])).toBe('/code/lumen');
  });

  it('siblings share parent', () => {
    expect(commonParent([
      '/code/lumen/lumen-app',
      '/code/lumen/lumen-canvas',
      '/code/lumen/lumen-shell',
    ])).toBe('/code/lumen');
  });

  it('different roots share deepest common', () => {
    expect(commonParent([
      '/code/lumen/app',
      '/code/brand/docs',
    ])).toBe('/code');
  });

  it('empty returns empty', () => {
    expect(commonParent([])).toBe('');
  });
});

// ── ScanWorkspaceState ─────────────────────────────────────────────────────────

describe('ScanWorkspaceState', () => {
  it('starts empty', () => {
    const state = new ScanWorkspaceState();
    expect(state.items).toEqual([]);
    expect(state.totalFolders).toBe(0);
  });

  // Audit item 15 (2026-10-28): the scan page recounted each workspace's
  // folders and files in its template — a second copy of the totals' rule.
  it('progressOf is one workspace\'s counts, and the totals are their sum', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'Lumen', [
      folder('f1', 'a', { filesTotal: 10, filesCompleted: 10, status: 'indexed' }),
      folder('f2', 'b', { filesTotal: 5, filesCompleted: 2, status: 'indexing' }),
    ]));
    state.add(workspace('p2', 'Other', [folder('f3', 'c', { filesTotal: 4, filesCompleted: 0, status: 'queued' })]));
    expect(state.progressOf(state.items[0]!)).toEqual({ folders: 2, ready: 1, files: 15, completed: 12 });
    expect([state.totalFolders, state.readyFolders, state.totalFiles, state.completedFiles]).toEqual([3, 1, 19, 12]);
  });

  it('add workspace', () => {
    const state = new ScanWorkspaceState();
    const p = workspace('p1', 'Lumen', [
      folder('f1', 'lumen-app', { stack: ['typescript'], filesTotal: 842 }),
      folder('f2', 'lumen-canvas', { stack: ['rust'], filesTotal: 614 }),
    ]);
    state.add(p);
    expect(state.items).toHaveLength(1);
    expect(state.totalFolders).toBe(2);
    expect(state.totalFiles).toBe(1456);
  });

  it('workspacePath derives from folders', () => {
    const state = new ScanWorkspaceState();
    const p = workspace('p1', 'Lumen', [
      folder('f1', 'lumen-app', { path: '/code/lumen/lumen-app' }),
      folder('f2', 'lumen-canvas', { path: '/code/lumen/lumen-canvas' }),
    ]);
    state.add(p);
    expect(state.workspacePath(state.items[0])).toBe('/code/lumen');
  });

  it('update merges folders by id', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'Lumen', [
      folder('f1', 'lumen-app', { filesTotal: 842, filesCompleted: 0, status: 'queued' }),
      folder('f2', 'lumen-canvas', { filesTotal: 614, filesCompleted: 0, status: 'queued' }),
    ]));

    // Partial update — only f1 changed
    state.update({ id: 'p1', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 400, status: 'indexing' },
    ] } as ScanWorkspace);

    const p = state.items[0];
    expect(p.folders).toHaveLength(2); // both folders preserved
    expect(p.folders.find(f => f.id === 'f1')!.filesCompleted).toBe(400);
    expect(p.folders.find(f => f.id === 'f2')!.filesCompleted).toBe(0); // unchanged
  });

  it('update merges workspace-level fields', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'Lumen', [], { status: 'scanning' }));

    state.update({ id: 'p1', status: 'active' } as ScanWorkspace);
    expect(state.items[0].status).toBe('active');
    expect(state.items[0].name).toBe('Lumen'); // unchanged
  });

  it('readyFolders counts indexed', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'Lumen', [
      folder('f1', 'app', { status: 'indexed' }),
      folder('f2', 'canvas', { status: 'indexed' }),
      folder('f3', 'shell', { status: 'indexing' }),
    ]));
    expect(state.readyFolders).toBe(2);
  });

  it('readyFolders counts folders with status indexed across workspaces', () => {
    const workspaces = new ScanWorkspaceState();
    workspaces.add({
      id: 'p1', name: 'Lumen', status: 'indexing', autoDetected: true, confidence: 'high',
      folders: [
        { id: 'f1', name: 'app',    path: '/code/lumen/app',    stack: ['rust'], filesTotal: 100, filesCompleted: 100, status: 'indexed' },
        { id: 'f2', name: 'canvas', path: '/code/lumen/canvas', stack: ['ts'],   filesTotal: 50,  filesCompleted: 25,  status: 'indexing' },
      ],
    });
    workspaces.add({
      id: 'p2', name: 'Other', status: 'indexing', autoDetected: true, confidence: 'high',
      folders: [
        { id: 'f3', name: 'core', path: '/code/other/core', stack: ['rust'], filesTotal: 10, filesCompleted: 10, status: 'indexed' },
      ],
    });
    expect(workspaces.readyFolders).toBe(2);
  });

  it('completedFiles sums across workspaces and folders', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'Lumen', [
      folder('f1', 'app', { filesTotal: 100, filesCompleted: 80 }),
      folder('f2', 'canvas', { filesTotal: 50, filesCompleted: 50 }),
    ]));
    state.add(workspace('p2', 'Brand', [
      folder('f3', 'docs', { filesTotal: 20, filesCompleted: 10 }),
    ]));
    expect(state.totalFiles).toBe(170);
    expect(state.completedFiles).toBe(140);
  });

  it('scanning is true when any workspace is scanning/indexing', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'A', [], { status: 'active' }));
    state.add(workspace('p2', 'B', [], { status: 'indexing' }));
    expect(state.scanning).toBe(true);
  });

  it('allWorkspacesResolved when all workspaces are active or failed', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'A', [], { status: 'active' }));
    state.add(workspace('p2', 'B', [], { status: 'failed' }));
    expect(state.allWorkspacesResolved).toBe(true);
  });

  it('allWorkspacesResolved is false when empty', () => {
    const state = new ScanWorkspaceState();
    expect(state.allWorkspacesResolved).toBe(false);
  });

  it('apply routes add event', () => {
    const state = new ScanWorkspaceState();
    state.apply({
      action: 'add', entity: 'workspace',
      data: workspace('p1', 'Lumen', [folder('f1', 'app')]),
    });
    expect(state.items).toHaveLength(1);
  });

  it('apply routes update event with folder merge', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'Lumen', [
      folder('f1', 'app', { filesTotal: 100, filesCompleted: 0 }),
      folder('f2', 'canvas', { filesTotal: 50, filesCompleted: 0 }),
    ]));

    state.update({
      id: 'p1', folders: [
        { id: 'f1', name: 'app', path: '/code/app', stack: [], filesTotal: 100, filesCompleted: 100, status: 'indexed' as const },
      ],
    } as unknown as ScanWorkspace);

    expect(state.items[0].folders.find(f => f.id === 'f1')!.filesCompleted).toBe(100);
    expect(state.items[0].folders.find(f => f.id === 'f2')!.filesCompleted).toBe(0);
  });

  it('remove workspace', () => {
    const state = new ScanWorkspaceState();
    state.add(workspace('p1', 'A', []));
    state.add(workspace('p2', 'B', []));
    state.remove({ id: 'p1' } as ScanWorkspace);
    expect(state.items).toHaveLength(1);
    expect(state.items[0].id).toBe('p2');
  });

  it('update merges status-only patches without overwriting name or folders', () => {
    const workspaces = new ScanWorkspaceState();
    workspaces.add({
      id: 'p1', name: 'Lumen', status: 'indexing', autoDetected: true, confidence: 'high',
      folders: [
        { id: 'f1', name: 'app', path: '/code/lumen/app', stack: ['rust'], filesTotal: 100, filesCompleted: 100, status: 'indexed' },
      ],
    });
    // Simulate daemon emitting a status-flip update with empty name + folders.
    workspaces.update({ id: 'p1', name: '', status: 'active', folders: [] } as any);
    expect(workspaces.items[0].name).toBe('Lumen');
    expect(workspaces.items[0].status).toBe('active');
    expect(workspaces.items[0].folders).toHaveLength(1);
  });
});

// ── ScanActivityState ────────────────────────────────────────────────────────

describe('ScanActivityState', () => {
  it('starts empty', () => {
    const state = new ScanActivityState();
    expect(state.items).toEqual([]);
    expect(state.totalElapsed).toBe(0);
  });

  it('recent returns newest first', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'discover', 'found root', 0.1));
    state.add(activity('a2', 'discover', 'found git repo', 0.2));
    state.add(activity('a3', 'queue', '842 files queued', 0.3));

    expect(state.recent[0].id).toBe('a3');
    expect(state.recent[2].id).toBe('a1');
  });

  it('totalElapsed from latest event', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'discover', 'found', 0.1));
    state.add(activity('a2', 'success', 'done', 1.5));
    expect(state.totalElapsed).toBe(1.5);
  });

  it('discovered counts every discover-level event', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'discover', '/code/lumen · git folder', 0.1));
    state.add(activity('a2', 'discover', '/code/lumen/app · git folder', 0.18));
    state.add(activity('a3', 'discover', '/code/canvas · standalone folder', 0.22));
    state.add(activity('a4', 'info', '3 git · 1 standalone workspace roots discovered', 0.30));
    expect(state.discovered).toBe(3);
  });

  it('queued counts queue events', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'queue', 'app · 842 files queued', 0.3));
    state.add(activity('a2', 'queue', 'canvas · 614 files queued', 0.4));
    state.add(activity('a3', 'process', 'app · 400/842', 0.5));
    expect(state.queued).toBe(2);
  });

  it('does not expose a processed counter — see ScanWorkspaceState folder statuses', () => {
    const state = new ScanActivityState();
    expect((state as any).processed).toBeUndefined();
  });

  it('scanComplete true when success event exists', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'process', 'done', 1.0));
    expect(state.scanComplete).toBe(false);

    state.add(activity('a2', 'success', 'scan complete · 21s', 1.2));
    expect(state.scanComplete).toBe(true);
  });

  it('apply adds activity events', () => {
    const state = new ScanActivityState();
    state.apply({
      action: 'add', entity: 'activity',
      data: activity('a1', 'discover', 'found root', 0.1),
    });
    expect(state.items).toHaveLength(1);
  });

  it('apply handles batch add', () => {
    const state = new ScanActivityState();
    state.apply({
      action: 'add', entity: 'activity',
      data: [
        activity('a1', 'discover', 'found root', 0.1),
        activity('a2', 'discover', 'found git repo', 0.2),
      ],
    });
    expect(state.items).toHaveLength(2);
  });
});

// ── Full scan simulation ─────────────────────────────────────────────────────

describe('scan simulation', () => {
  it('simulates a complete scan flow', () => {
    const workspaces = new ScanWorkspaceState();
    const activities = new ScanActivityState();

    // Phase 1: Discovery
    activities.apply({ action: 'add', entity: 'activity', data: activity('a01', 'discover', '~/code/lumen · found root', 0.12) });
    activities.apply({ action: 'add', entity: 'activity', data: activity('a02', 'discover', '~/code/lumen/lumen-app · found git repo', 0.18) });
    activities.apply({ action: 'add', entity: 'activity', data: activity('a03', 'discover', '~/code/lumen/lumen-canvas · found git repo', 0.24) });

    expect(activities.discovered).toBe(3);

    // Phase 2: Workspace detected
    workspaces.apply({ action: 'add', entity: 'workspace', data: workspace('p1', 'Lumen Studio', [
      folder('f1', 'lumen-app', { path: '/code/lumen/lumen-app', stack: ['typescript'] }),
      folder('f2', 'lumen-canvas', { path: '/code/lumen/lumen-canvas', stack: ['rust'] }),
    ]) });

    expect(workspaces.items).toHaveLength(1);
    expect(workspaces.workspacePath(workspaces.items[0])).toBe('/code/lumen');

    // Phase 3: Queued
    activities.apply({ action: 'add', entity: 'activity', data: activity('a04', 'queue', 'lumen-app · 842 files queued', 0.38) });
    activities.apply({ action: 'add', entity: 'activity', data: activity('a05', 'queue', 'lumen-canvas · 614 files queued', 0.42) });

    workspaces.apply({ action: 'update', entity: 'workspace', data: { id: 'p1', status: 'indexing', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 0, status: 'queued' as const },
      { id: 'f2', name: 'lumen-canvas', path: '/code/lumen/lumen-canvas', stack: ['rust'], filesTotal: 614, filesCompleted: 0, status: 'queued' as const },
    ] } as ScanWorkspace });

    expect(activities.queued).toBe(2);
    expect(workspaces.totalFiles).toBe(1456);

    // Phase 4: Processing
    workspaces.apply({ action: 'update', entity: 'workspace', data: { id: 'p1', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 612, status: 'indexing' as const },
    ] } as ScanWorkspace });

    expect(workspaces.completedFiles).toBe(612); // f1=612, f2=0

    // Phase 5: Complete
    workspaces.apply({ action: 'update', entity: 'workspace', data: { id: 'p1', status: 'active', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 842, status: 'indexed' as const },
      { id: 'f2', name: 'lumen-canvas', path: '/code/lumen/lumen-canvas', stack: ['rust'], filesTotal: 614, filesCompleted: 614, status: 'indexed' as const },
    ] } as ScanWorkspace });

    activities.apply({ action: 'add', entity: 'activity', data: activity('a10', 'success', 'scan complete · 21s', 1.26) });

    expect(workspaces.allWorkspacesResolved).toBe(true);
    expect(workspaces.completedFiles).toBe(1456);
    expect(workspaces.readyFolders).toBe(2);
    expect(activities.scanComplete).toBe(true);
    expect(activities.totalElapsed).toBe(1.26);
  });
});

// #288: the scan's done signal. A status read used to fall back to an empty
// queue, so a daemon that stopped answering mid-scan read as reachable AND
// idle — two such polls marked the scan done.
describe('pollStep', () => {
  const status = (pending: number, running: number) => ({
    ok: true as const,
    data: { queue: { pending, blocked: 0, running, completed: 0 }, repos: {} },
  });
  const down = { ok: false as const, error: { status: 0, message: 'ECONNREFUSED' } };

  it('counts an idle queue; two in a row are done', () => {
    expect(pollStep(status(0, 0), 0)).toEqual({ reachable: true, idlePolls: 1, done: false });
    expect(pollStep(status(0, 0), 1)).toEqual({ reachable: true, idlePolls: 2, done: true });
  });
  it('a busy queue resets the count', () => {
    expect(pollStep(status(3, 1), 1)).toEqual({ reachable: true, idlePolls: 0, done: false });
  });
  it('a failed read is unreachable — never idle, never done', () => {
    expect(pollStep(down, 1)).toEqual({ reachable: false, idlePolls: 0, done: false });
  });
});

// #304: scanFolder went through the legacy post, so a root the daemon refused
// to scan was `{ ok: false }`, ignored — the poller then called an idle queue
// "done" over a scan that never started.
describe('triggerScans', () => {
  const ok = { ok: true as const, data: { ok: true, scanning: true } };
  const root = (path: string, scanned: boolean) => ({ path, scanned });

  it('scans only the roots not yet scanned', async () => {
    const triggerScan = vi.fn(async (_path: string) => ok);
    await triggerScans({ triggerScan }, [root('/a', false), root('/b', true), root('/c', false)]);
    expect(triggerScan.mock.calls.map((c) => c[0])).toEqual(['/a', '/c']);
  });

  it('a refused scan throws, naming each root and why', async () => {
    const triggerScan = vi.fn(async (path: string) =>
      path === '/a' ? ok : { ok: false as const, error: { status: 503, message: 'Service Unavailable' } },
    );
    await expect(triggerScans({ triggerScan }, [root('/a', false), root('/c', false)]))
      .rejects.toThrow(/\/c.*Service Unavailable/);
    expect(triggerScan).toHaveBeenCalledTimes(2);
  });
});

// ── miniSummary ──────────────────────────────────────────────────────────────
// The mini band's one line: file counts when the stream has delivered any,
// queue counts always. "Queued" is pending + running — everything still in
// the queue, whatever its phase.

describe('miniSummary', () => {
  const snap = (pending: number, running: number, completed: number): QueueCounts => ({ pending, running, completed });

  it('files plus queue when totals are known', () => {
    expect(miniSummary(snap(80, 6, 12), 1240, 3580)).toBe('Indexing 1,240/3,580 files · 86 queued');
  });

  it('queue only before the first file counts arrive', () => {
    expect(miniSummary(snap(80, 6, 12), 0, 0)).toBe('Indexing · 86 queued');
  });

  it('a bare line when nothing is known yet', () => {
    expect(miniSummary(null, 0, 0)).toBe('Indexing…');
    expect(miniSummary(snap(0, 0, 0), 0, 0)).toBe('Indexing…');
  });

  it('no snapshot, no queue claim — files alone', () => {
    expect(miniSummary(null, 1240, 3580)).toBe('Indexing 1,240/3,580 files');
  });
});

// ── ScanState.attach / .start ────────────────────────────────────────────────
// The footer observes through a canned source: scripted queue reads, a
// captured stream, and spies for every write. Fake timers drive the poller.

type Queue = { pending: number; running: number; completed: number };
const BUSY: Queue = { pending: 80, running: 6, completed: 12 };
const IDLE: Queue = { pending: 0, running: 0, completed: 107 };

function cannedSource(script: { queues?: (Queue | 'down')[]; roots?: { path: string; scanned: boolean }[] } = {}) {
  const queues = script.queues ?? [IDLE];
  let qi = 0;
  const stream: {
    on: ((e: ScanStreamEvent) => void) | null;
    onOpen: (() => void) | null;
    subscribes: number;
    teardowns: number;
  } = { on: null, onOpen: null, subscribes: 0, teardowns: 0 };
  const read = (q: Queue | 'down'): ApiResult<IndexQueueStatus> =>
    q === 'down'
      ? { ok: false, error: { status: 0, message: 'ECONNREFUSED' } }
      : { ok: true, data: { queue: { ...q, blocked: 0 } } };
  const source: ScanSource = {
    readWorkspaces: vi.fn(async () => ({ ok: true as const, data: [] })),
    readRoots: vi.fn(async () => ({ ok: true as const, data: script.roots ?? [] })),
    triggerScan: vi.fn(async (_root: string) => ({ ok: true as const, data: { ok: true, scanning: true } })),
    readIndexStatus: vi.fn(async () => read(queues[Math.min(qi++, queues.length - 1)]!)),
    events: ((on: (e: ScanStreamEvent) => void, onOpen: () => void) => {
      stream.subscribes += 1;
      stream.on = on;
      stream.onOpen = onOpen;
      onOpen();
      return () => {
        stream.teardowns += 1;
      };
    }) as StateStream<ScanStreamEvent>,
  };
  return { source, stream };
}

describe('ScanState.attach — passive observation', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('a busy queue reads as scanning, stores the snapshot, and triggers nothing', async () => {
    const { source } = cannedSource({ queues: [BUSY] });
    const st = new ScanState(source);
    await st.attach();
    expect(st.status).toBe('scanning');
    expect(st.queueSnapshot).toEqual(BUSY);
    expect(source.triggerScan).not.toHaveBeenCalled();
    expect(source.readWorkspaces).toHaveBeenCalledTimes(1);
    st.reset();
  });

  it('an idle queue stays idle — never done', async () => {
    const { source } = cannedSource({ queues: [IDLE] });
    const st = new ScanState(source);
    await st.attach();
    expect(st.status).toBe('idle');
    await vi.advanceTimersByTimeAsync(3200);
    expect(st.status).toBe('idle');
    expect(st.completed).toBe(false);
    st.reset();
  });

  it('a draining queue returns to idle, and the poller keeps watching', async () => {
    const { source } = cannedSource({ queues: [BUSY, IDLE, IDLE, IDLE] });
    const st = new ScanState(source);
    await st.attach();
    expect(st.status).toBe('scanning');
    await vi.advanceTimersByTimeAsync(3200);
    expect(st.status).toBe('idle');
    const reads = (source.readIndexStatus as ReturnType<typeof vi.fn>).mock.calls.length;
    await vi.advanceTimersByTimeAsync(16000);
    expect((source.readIndexStatus as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(reads);
    st.reset();
  });

  it('a queue that starts later flips idle to scanning', async () => {
    const { source } = cannedSource({ queues: [IDLE, BUSY] });
    const st = new ScanState(source);
    await st.attach();
    expect(st.status).toBe('idle');
    await vi.advanceTimersByTimeAsync(1600);
    expect(st.status).toBe('scanning');
    st.reset();
  });

  it('a dead daemon reads as unreachable and idle, and recovery rehydrates', async () => {
    const { source } = cannedSource({ queues: [BUSY] });
    const down = { ok: false as const, error: { status: 0, message: 'ECONNREFUSED' } };
    source.readWorkspaces = vi
      .fn()
      .mockResolvedValueOnce(down)
      .mockResolvedValue({ ok: true, data: [] });
    const st = new ScanState(source);
    await st.attach();
    expect(st.daemonReachable).toBe(false);
    expect(st.status).toBe('idle');
    await vi.advanceTimersByTimeAsync(1600);
    expect(st.daemonReachable).toBe(true);
    expect(st.status).toBe('scanning');
    expect(source.readWorkspaces).toHaveBeenCalledTimes(2);
    st.reset();
  });

  it('stream events dispatch into the stores', async () => {
    const { source, stream } = cannedSource({ queues: [BUSY] });
    const st = new ScanState(source);
    await st.attach();
    // A reconnect open re-reads (broadcast has no replay).
    const hydrateCalls = (source.readWorkspaces as ReturnType<typeof vi.fn>).mock.calls.length;
    stream.onOpen!();
    expect((source.readWorkspaces as ReturnType<typeof vi.fn>).mock.calls.length).toBe(hydrateCalls + 1);
    stream.on!({ action: 'add', entity: 'activity', data: activity('a1', 'discover', 'found root', 0.1) });
    expect(st.activities.items).toHaveLength(1);
    stream.on!({ action: 'add', entity: 'workspace', data: workspace('p1', 'Lumen', [folder('f1', 'app')]) });
    expect(st.workspaces.items).toHaveLength(1);
    st.reset();
  });

  it('attach is idempotent while attached', async () => {
    const { source, stream } = cannedSource({ queues: [BUSY] });
    const st = new ScanState(source);
    await st.attach();
    await st.attach();
    expect(stream.subscribes).toBe(1);
    st.reset();
  });

  it('reset tears down the stream and clears the run, but keeps expanded', async () => {
    const { source, stream } = cannedSource({ queues: [BUSY] });
    const st = new ScanState(source);
    st.expanded = true;
    await st.attach();
    st.reset();
    expect(stream.teardowns).toBe(1);
    expect(st.status).toBe('idle');
    expect(st.queueSnapshot).toBeNull();
    expect(st.expanded).toBe(true);
  });
});

describe('ScanState.start — owning a run', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('triggers unscanned roots and reaches done on an idle queue', async () => {
    const { source } = cannedSource({
      queues: [IDLE],
      roots: [
        { path: '/a', scanned: false },
        { path: '/b', scanned: true },
      ],
    });
    const st = new ScanState(source);
    await st.start();
    expect(st.status).toBe('scanning');
    expect((source.triggerScan as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0])).toEqual(['/a']);
    await vi.advanceTimersByTimeAsync(3200);
    expect(st.status).toBe('done');
    expect(st.completed).toBe(true);
    const reads = (source.readIndexStatus as ReturnType<typeof vi.fn>).mock.calls.length;
    await vi.advanceTimersByTimeAsync(30000);
    expect((source.readIndexStatus as ReturnType<typeof vi.fn>).mock.calls.length).toBe(reads);
    st.reset();
  });

  it('start upgrades a passive observation instead of resubscribing', async () => {
    const { source, stream } = cannedSource({ queues: [BUSY], roots: [{ path: '/a', scanned: false }] });
    const st = new ScanState(source);
    await st.attach();
    await st.start();
    expect(stream.subscribes).toBe(1);
    expect(source.triggerScan).toHaveBeenCalledTimes(1);
    st.reset();
  });

  it('concurrent attach and start share one subscription and one trigger', async () => {
    const { source, stream } = cannedSource({ queues: [BUSY], roots: [{ path: '/a', scanned: false }] });
    const st = new ScanState(source);
    await Promise.all([st.attach(), st.start()]);
    expect(stream.subscribes).toBe(1);
    expect(source.triggerScan).toHaveBeenCalledTimes(1);
    expect(st.status).toBe('scanning');
    st.reset();
  });

  it('a refused trigger fails the run and stops the poller', async () => {
    const { source } = cannedSource({ queues: [IDLE], roots: [{ path: '/a', scanned: false }] });
    source.triggerScan = vi.fn(async () => ({ ok: false as const, error: { status: 503, message: 'Service Unavailable' } }));
    const st = new ScanState(source);
    await st.start();
    expect(st.status).toBe('error');
    expect(st.error).toMatch(/\/a.*Service Unavailable/);
    const reads = (source.readIndexStatus as ReturnType<typeof vi.fn>).mock.calls.length;
    await vi.advanceTimersByTimeAsync(30000);
    expect((source.readIndexStatus as ReturnType<typeof vi.fn>).mock.calls.length).toBe(reads);
    st.reset();
  });
});
