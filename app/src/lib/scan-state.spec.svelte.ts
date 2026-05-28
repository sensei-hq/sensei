import { describe, it, expect } from 'vitest';
import { ScanProjectState, ScanActivityState, commonParent } from './scan-state.svelte.js';
import type { ScanProject, ScanProjectFolder, ActivityEvent } from './types.js';

// ── Factories ────────────────────────────────────────────────────────────────

function folder(id: string, name: string, opts: Partial<ScanProjectFolder> = {}): ScanProjectFolder {
  return { id, name, path: `/code/${name}`, stack: [], filesTotal: 0, filesCompleted: 0, status: 'discovered', ...opts };
}

function project(id: string, name: string, folders: ScanProjectFolder[], opts: Partial<ScanProject> = {}): ScanProject {
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

// ── ScanProjectState ─────────────────────────────────────────────────────────

describe('ScanProjectState', () => {
  it('starts empty', () => {
    const state = new ScanProjectState();
    expect(state.items).toEqual([]);
    expect(state.totalFolders).toBe(0);
  });

  it('add project', () => {
    const state = new ScanProjectState();
    const p = project('p1', 'Lumen', [
      folder('f1', 'lumen-app', { stack: ['typescript'], filesTotal: 842 }),
      folder('f2', 'lumen-canvas', { stack: ['rust'], filesTotal: 614 }),
    ]);
    state.add(p);
    expect(state.items).toHaveLength(1);
    expect(state.totalFolders).toBe(2);
    expect(state.totalFiles).toBe(1456);
  });

  it('projectPath derives from folders', () => {
    const state = new ScanProjectState();
    const p = project('p1', 'Lumen', [
      folder('f1', 'lumen-app', { path: '/code/lumen/lumen-app' }),
      folder('f2', 'lumen-canvas', { path: '/code/lumen/lumen-canvas' }),
    ]);
    state.add(p);
    expect(state.projectPath(state.items[0])).toBe('/code/lumen');
  });

  it('update merges folders by id', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'Lumen', [
      folder('f1', 'lumen-app', { filesTotal: 842, filesCompleted: 0, status: 'queued' }),
      folder('f2', 'lumen-canvas', { filesTotal: 614, filesCompleted: 0, status: 'queued' }),
    ]));

    // Partial update — only f1 changed
    state.update({ id: 'p1', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 400, status: 'indexing' },
    ] } as ScanProject);

    const p = state.items[0];
    expect(p.folders).toHaveLength(2); // both folders preserved
    expect(p.folders.find(f => f.id === 'f1')!.filesCompleted).toBe(400);
    expect(p.folders.find(f => f.id === 'f2')!.filesCompleted).toBe(0); // unchanged
  });

  it('update merges project-level fields', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'Lumen', [], { status: 'scanning' }));

    state.update({ id: 'p1', status: 'active' } as ScanProject);
    expect(state.items[0].status).toBe('active');
    expect(state.items[0].name).toBe('Lumen'); // unchanged
  });

  it('readyFolders counts indexed', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'Lumen', [
      folder('f1', 'app', { status: 'indexed' }),
      folder('f2', 'canvas', { status: 'indexed' }),
      folder('f3', 'shell', { status: 'indexing' }),
    ]));
    expect(state.readyFolders).toBe(2);
  });

  it('completedFiles sums across projects and folders', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'Lumen', [
      folder('f1', 'app', { filesTotal: 100, filesCompleted: 80 }),
      folder('f2', 'canvas', { filesTotal: 50, filesCompleted: 50 }),
    ]));
    state.add(project('p2', 'Brand', [
      folder('f3', 'docs', { filesTotal: 20, filesCompleted: 10 }),
    ]));
    expect(state.totalFiles).toBe(170);
    expect(state.completedFiles).toBe(140);
  });

  it('scanning is true when any project is scanning/indexing', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'A', [], { status: 'active' }));
    state.add(project('p2', 'B', [], { status: 'indexing' }));
    expect(state.scanning).toBe(true);
  });

  it('allProjectsResolved when all projects are active or failed', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'A', [], { status: 'active' }));
    state.add(project('p2', 'B', [], { status: 'failed' }));
    expect(state.allProjectsResolved).toBe(true);
  });

  it('allProjectsResolved is false when empty', () => {
    const state = new ScanProjectState();
    expect(state.allProjectsResolved).toBe(false);
  });

  it('apply routes add event', () => {
    const state = new ScanProjectState();
    state.apply({
      action: 'add', entity: 'project',
      data: project('p1', 'Lumen', [folder('f1', 'app')]),
    });
    expect(state.items).toHaveLength(1);
  });

  it('apply routes update event with folder merge', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'Lumen', [
      folder('f1', 'app', { filesTotal: 100, filesCompleted: 0 }),
      folder('f2', 'canvas', { filesTotal: 50, filesCompleted: 0 }),
    ]));

    state.update({
      id: 'p1', folders: [
        { id: 'f1', name: 'app', path: '/code/app', stack: [], filesTotal: 100, filesCompleted: 100, status: 'indexed' as const },
      ],
    } as unknown as ScanProject);

    expect(state.items[0].folders.find(f => f.id === 'f1')!.filesCompleted).toBe(100);
    expect(state.items[0].folders.find(f => f.id === 'f2')!.filesCompleted).toBe(0);
  });

  it('remove project', () => {
    const state = new ScanProjectState();
    state.add(project('p1', 'A', []));
    state.add(project('p2', 'B', []));
    state.remove({ id: 'p1' } as ScanProject);
    expect(state.items).toHaveLength(1);
    expect(state.items[0].id).toBe('p2');
  });

  it('update merges status-only patches without overwriting name or folders', () => {
    const projects = new ScanProjectState();
    projects.add({
      id: 'p1', name: 'Lumen', status: 'indexing', autoDetected: true, confidence: 'high',
      folders: [
        { id: 'f1', name: 'app', path: '/code/lumen/app', stack: ['rust'], filesTotal: 100, filesCompleted: 100, status: 'indexed' },
      ],
    });
    // Simulate daemon emitting a status-flip update with empty name + folders.
    projects.update({ id: 'p1', name: '', status: 'active', folders: [] } as any);
    expect(projects.items[0].name).toBe('Lumen');
    expect(projects.items[0].status).toBe('active');
    expect(projects.items[0].folders).toHaveLength(1);
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
    state.add(activity('a4', 'info', '3 git · 0 sibling · 1 standalone folders discovered', 0.30));
    expect(state.discovered).toBe(3);
  });

  it('queued counts queue events', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'queue', 'app · 842 files queued', 0.3));
    state.add(activity('a2', 'queue', 'canvas · 614 files queued', 0.4));
    state.add(activity('a3', 'process', 'app · 400/842', 0.5));
    expect(state.queued).toBe(2);
  });

  it('does not expose a processed counter — see ScanProjectState folder statuses', () => {
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
    const projects = new ScanProjectState();
    const activities = new ScanActivityState();

    // Phase 1: Discovery
    activities.apply({ action: 'add', entity: 'activity', data: activity('a01', 'discover', '~/code/lumen · found root', 0.12) });
    activities.apply({ action: 'add', entity: 'activity', data: activity('a02', 'discover', '~/code/lumen/lumen-app · found git repo', 0.18) });
    activities.apply({ action: 'add', entity: 'activity', data: activity('a03', 'discover', '~/code/lumen/lumen-canvas · found git repo', 0.24) });

    expect(activities.discovered).toBe(3);

    // Phase 2: Project detected
    projects.apply({ action: 'add', entity: 'project', data: project('p1', 'Lumen Studio', [
      folder('f1', 'lumen-app', { path: '/code/lumen/lumen-app', stack: ['typescript'] }),
      folder('f2', 'lumen-canvas', { path: '/code/lumen/lumen-canvas', stack: ['rust'] }),
    ]) });

    expect(projects.items).toHaveLength(1);
    expect(projects.projectPath(projects.items[0])).toBe('/code/lumen');

    // Phase 3: Queued
    activities.apply({ action: 'add', entity: 'activity', data: activity('a04', 'queue', 'lumen-app · 842 files queued', 0.38) });
    activities.apply({ action: 'add', entity: 'activity', data: activity('a05', 'queue', 'lumen-canvas · 614 files queued', 0.42) });

    projects.apply({ action: 'update', entity: 'project', data: { id: 'p1', status: 'indexing', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 0, status: 'queued' as const },
      { id: 'f2', name: 'lumen-canvas', path: '/code/lumen/lumen-canvas', stack: ['rust'], filesTotal: 614, filesCompleted: 0, status: 'queued' as const },
    ] } as ScanProject });

    expect(activities.queued).toBe(2);
    expect(projects.totalFiles).toBe(1456);

    // Phase 4: Processing
    projects.apply({ action: 'update', entity: 'project', data: { id: 'p1', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 612, status: 'indexing' as const },
    ] } as ScanProject });

    expect(projects.completedFiles).toBe(612); // f1=612, f2=0

    // Phase 5: Complete
    projects.apply({ action: 'update', entity: 'project', data: { id: 'p1', status: 'active', folders: [
      { id: 'f1', name: 'lumen-app', path: '/code/lumen/lumen-app', stack: ['typescript'], filesTotal: 842, filesCompleted: 842, status: 'indexed' as const },
      { id: 'f2', name: 'lumen-canvas', path: '/code/lumen/lumen-canvas', stack: ['rust'], filesTotal: 614, filesCompleted: 614, status: 'indexed' as const },
    ] } as ScanProject });

    activities.apply({ action: 'add', entity: 'activity', data: activity('a10', 'success', 'scan complete · 21s', 1.26) });

    expect(projects.allProjectsResolved).toBe(true);
    expect(projects.completedFiles).toBe(1456);
    expect(projects.readyFolders).toBe(2);
    expect(activities.scanComplete).toBe(true);
    expect(activities.totalElapsed).toBe(1.26);
  });
});
