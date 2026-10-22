import { describe, it, expect } from 'vitest';
import { ScanWorkspaceState, ScanActivityState, commonParent, pollStep } from './scan-state.svelte.js';
import type { ScanWorkspace, ScanWorkspaceFolder, ActivityEvent } from './types.js';

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
