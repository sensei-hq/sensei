// Unit tests for the dōjō's client-side CodeMapPort (`graph-port-client.ts`,
// #268): the /v1/you/graph reads behind the port interface, transport
// failures carried as their reason, cochange/complexity answered locally
// (repo-independent: no history anywhere, no Measures anywhere).
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({
	getGraphLevel0: vi.fn(),
	getGraphModule: vi.fn(),
	getGraphFile: vi.fn(),
	getGraphZones: vi.fn()
}));

vi.mock('./client-data', () => ({
	getGraphLevel0: mocks.getGraphLevel0,
	getGraphModule: mocks.getGraphModule,
	getGraphFile: mocks.getGraphFile,
	getGraphZones: mocks.getGraphZones
}));

import { graphPortClient } from './graph-port-client';

const ctx = () => ({ repoKey: 'github.com/acme/api', branch: 'main' });

describe('graphPortClient', () => {
	beforeEach(() => vi.clearAllMocks());

	it('level0/module/file/zones call through with the repo context', async () => {
		mocks.getGraphLevel0.mockResolvedValue({ ok: true, data: { level: 'module' } });
		mocks.getGraphModule.mockResolvedValue({ ok: true, data: { level: 'file' } });
		mocks.getGraphFile.mockResolvedValue({ ok: true, data: { level: 'symbol' } });
		mocks.getGraphZones.mockResolvedValue({ ok: true, data: { points: [] } });
		const port = graphPortClient(ctx);
		expect(await port.level0('folders', null)).toEqual({ ok: true, data: { level: 'module' } });
		expect(mocks.getGraphLevel0).toHaveBeenCalledWith(expect.objectContaining({ repoKey: 'github.com/acme/api', branch: 'main', groupBy: 'folders' }));
		expect((await port.module('shop')).ok).toBe(true);
		expect((await port.file('a.ts')).ok).toBe(true);
		expect((await port.zones()).ok).toBe(true);
	});

	it('a transport failure is the reason, never thrown', async () => {
		mocks.getGraphLevel0.mockRejectedValue(new Error('the request failed'));
		expect(await graphPortClient(ctx).level0('layers', null)).toEqual({ ok: false, error: 'the request failed' });
	});

	it('cochange answers no history locally; complexity says Measures are not synced', async () => {
		const port = graphPortClient(ctx);
		expect(await port.cochange()).toEqual({ ok: true, data: { hidden: [], pairs: 0, commits: 0, minTogether: 5 } });
		expect(mocks.getGraphLevel0).not.toHaveBeenCalled();
		const c = await port.complexity();
		expect(c.ok).toBe(false);
		if (c.ok) return;
		expect(c.error).toMatch(/not synced/);
	});
});
