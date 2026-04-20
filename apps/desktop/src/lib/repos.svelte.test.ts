import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { RepoStore } from './repos.svelte.js';
import { flushSync } from 'svelte';

/**
 * Integration test for RepoStore.
 * Requires senseid running on :7744 with at least 1 indexed project.
 * Run: cd apps/desktop && bun run vitest run
 */

const DAEMON_PORT = 7744;

function daemonAvailable(): Promise<boolean> {
  return fetch(`http://127.0.0.1:${DAEMON_PORT}/health`)
    .then(r => r.ok)
    .catch(() => false);
}

async function getProjectCount(): Promise<number> {
  const r = await fetch(`http://127.0.0.1:${DAEMON_PORT}/api/projects`);
  const data = await r.json();
  return data.length;
}

describe('RepoStore', () => {
  let store: RepoStore;
  let available = false;

  beforeAll(async () => {
    available = await daemonAvailable();
    if (!available) return;
    store = new RepoStore(DAEMON_PORT);
  });

  afterAll(() => {
    store?.disconnect();
  });

  it('should initialize with empty state', () => {
    const s = new RepoStore(DAEMON_PORT);
    expect(s.all).toEqual([]);
    expect(s.totalCount).toBe(0);
    expect(s.indexedCount).toBe(0);
    expect(s.anyIndexing).toBe(false);
    expect(s.search).toBe('');
  });

  it('should fetch projects from daemon on connect', async () => {
    if (!available) return;

    const count = await getProjectCount();
    if (count === 0) return; // skip if no projects

    store.connect();

    // Wait for initial fetch
    await new Promise(r => setTimeout(r, 1000));
    flushSync();

    expect(store.totalCount).toBeGreaterThan(0);
    expect(store.all.length).toBe(count);

    // Each entry should have project data
    for (const entry of store.all) {
      expect(entry.project.repo_id).toBeTruthy();
      expect(entry.project.name).toBeTruthy();
      expect(entry.project.path).toBeTruthy();
      expect(['idle', 'queued', 'indexing', 'indexed', 'failed']).toContain(entry.indexState);
    }
  });

  it('should derive indexedCount from all entries', async () => {
    if (!available || store.totalCount === 0) return;

    flushSync();
    const manualCount = store.all.filter(r => r.indexState === 'indexed').length;
    expect(store.indexedCount).toBe(manualCount);
  });

  it('should filter repos by search', async () => {
    if (!available || store.totalCount === 0) return;

    const firstName = store.all[0].project.name;

    store.search = firstName;
    flushSync();

    expect(store.repos.length).toBeGreaterThan(0);
    expect(store.repos.every(r => r.project.name.toLowerCase().includes(firstName.toLowerCase()))).toBe(true);

    // Clear search
    store.search = '';
    flushSync();
    expect(store.repos.length).toBe(store.totalCount);
  });

  it('should sort indexing repos before indexed', async () => {
    if (!available || store.totalCount < 2) return;

    flushSync();

    // If any are indexing, they should be first
    let seenNonActive = false;
    for (const r of store.repos) {
      const isActive = r.indexState === 'indexing' || r.indexState === 'queued';
      if (!isActive) seenNonActive = true;
      if (isActive && seenNonActive) {
        throw new Error('Indexing repo found after non-indexing repo — sort is wrong');
      }
    }
  });

  it('should have updatedAt for recency sort', async () => {
    if (!available || store.totalCount === 0) return;

    flushSync();
    for (const r of store.repos) {
      expect(r.updatedAt).toBeGreaterThan(0);
    }
  });

  it('should expose derived aggregate counts', async () => {
    if (!available) return;

    flushSync();
    expect(store.totalCount).toBe(store.all.length);
    expect(store.indexedCount).toBeLessThanOrEqual(store.totalCount);
    expect(store.indexingCount).toBeLessThanOrEqual(store.totalCount);
    expect(typeof store.anyIndexing).toBe('boolean');
    expect(store.totalFiles).toBeGreaterThanOrEqual(0);
    expect(store.completedFiles).toBeGreaterThanOrEqual(0);
  });
});
