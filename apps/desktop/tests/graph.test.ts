import { test, expect } from '@playwright/test';

/**
 * Graph visualization tests — verify that the new hierarchy
 * (packages, modules, HAS_METHOD, CONTAINS_* edges) renders correctly.
 *
 * Prerequisites:
 *   - senseid daemon running on port 7744
 *   - at least one project indexed (e.g. "sensei")
 */

const API = 'http://127.0.0.1:7744';

test.describe('Graph API — hierarchy nodes and edges', () => {
  test('graph/nodes returns package and module kinds', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    expect(res.ok()).toBe(true);
    const data = await res.json();
    const kinds = new Set(data.nodes.map((n: { kind: string }) => n.kind));
    expect(kinds.has('function')).toBe(true);
    expect(kinds.has('package')).toBe(true);
    expect(kinds.has('module')).toBe(true);
  });

  test('graph/nodes returns HAS_METHOD edges', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    expect(res.ok()).toBe(true);
    const data = await res.json();
    const edgeTypes = new Set(data.edges.map((e: { type: string }) => e.type));
    expect(edgeTypes.has('HAS_METHOD')).toBe(true);
  });

  test('graph/nodes returns CONTAINS_FN edges', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    expect(res.ok()).toBe(true);
    const data = await res.json();
    const edgeTypes = new Set(data.edges.map((e: { type: string }) => e.type));
    expect(edgeTypes.has('CONTAINS_FN')).toBe(true);
  });

  test('graph/nodes returns CONTAINS_MOD edges', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    expect(res.ok()).toBe(true);
    const data = await res.json();
    const edgeTypes = new Set(data.edges.map((e: { type: string }) => e.type));
    expect(edgeTypes.has('CONTAINS_MOD')).toBe(true);
  });

  test('package nodes have correct id format', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    const data = await res.json();
    const pkgNodes = data.nodes.filter((n: { kind: string }) => n.kind === 'package');
    expect(pkgNodes.length).toBeGreaterThan(0);
    for (const pkg of pkgNodes) {
      expect(pkg.id).toMatch(/^pkg:/);
    }
  });

  test('module nodes have correct id format', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    const data = await res.json();
    const modNodes = data.nodes.filter((n: { kind: string }) => n.kind === 'module');
    expect(modNodes.length).toBeGreaterThan(0);
    for (const mod of modNodes) {
      expect(mod.id).toMatch(/^mod:/);
    }
  });

  test('HAS_METHOD edges link type to function', async ({ request }) => {
    const res = await request.get(`${API}/api/graph/nodes?repoId=sensei`);
    const data = await res.json();
    const nodeIds = new Set(data.nodes.map((n: { id: string }) => n.id));
    const hasMethodEdges = data.edges.filter((e: { type: string }) => e.type === 'HAS_METHOD');
    expect(hasMethodEdges.length).toBeGreaterThan(0);
    // Source should be a type node, target a function node
    for (const edge of hasMethodEdges.slice(0, 10)) {
      expect(edge.source).toMatch(/^type:/);
      expect(edge.target).toMatch(/^fn:/);
    }
  });
});

test.describe('Project page — graph visualization', () => {
  test('project page loads graph canvas', async ({ page }) => {
    await page.goto('/p/sensei');
    await page.waitForLoadState('networkidle');
    // Page should have loaded — heading shows project name
    await expect(page.getByRole('heading', { name: 'sensei' })).toBeVisible({ timeout: 15_000 });
    // Canvas element should exist for the graph
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible({ timeout: 10_000 });
  });

  test('project page shows stats including edges', async ({ page }) => {
    await page.goto('/p/sensei');
    await page.waitForLoadState('networkidle');
    // Stats section — use exact text match for uppercase labels
    await expect(page.getByText('Functions', { exact: true })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Types', { exact: true })).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('Edges', { exact: true })).toBeVisible({ timeout: 10_000 });
  });

  test('graph canvas is rendered with non-zero dimensions', async ({ page }) => {
    await page.goto('/p/sensei');
    await page.waitForLoadState('networkidle');
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible({ timeout: 15_000 });
    const box = await canvas.boundingBox();
    expect(box).toBeTruthy();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);
  });
});

test.describe('All projects page', () => {
  test('projects list loads', async ({ page }) => {
    await page.goto('/all');
    // Don't wait for networkidle — re-indexing creates ongoing SSE connections
    const projectLinks = page.locator('a[href^="/p/"]');
    await expect(projectLinks.first()).toBeVisible({ timeout: 30_000 });
    const count = await projectLinks.count();
    expect(count).toBeGreaterThan(0);
  });
});
