import { beforeEach, describe, expect, it, vi } from 'vitest';

// Stub the appstate module so importing the mcp store doesn't drag in
// the whole Tauri appstate.svelte pipeline.
vi.mock('$lib/appstate.svelte.js', () => ({
    appState: { port: 7744 },
}));

// Stub senseiApi with call trackers we assert against later.
const mocks = {
    mcpListTools: vi.fn(),
    getToolUsage: vi.fn(),
    getToolSignals: vi.fn(),
    getToolInsights: vi.fn(),
    getSessionToolTimeline: vi.fn(),
};

vi.mock('$lib/api.js', () => ({
    senseiApi: () => mocks,
}));

// Re-import fresh for each test so the module-level $state resets.
async function freshStore() {
    vi.resetModules();
    const { mcp } = await import('./mcp.svelte.js');
    return mcp;
}

beforeEach(() => {
    for (const key of Object.keys(mocks)) {
        (mocks as Record<string, ReturnType<typeof vi.fn>>)[key].mockReset();
    }
    mocks.mcpListTools.mockResolvedValue({ tools: [{ mcp: 'sensei', id: 'sensei.search', name: 'search', kind: 'query', summary: 's', inputs: [], example: { response: '' } }] });
    mocks.getToolUsage.mockResolvedValue({ tools: [] });
    mocks.getToolSignals.mockResolvedValue({ signals: [], source: 'cache' });
    mocks.getToolInsights.mockResolvedValue({ insights: [{ toolName: 'search', computedAt: '2026-07-02T12:00:00Z', metrics: {}, variant: 'win', title: 'Workhorse', detail: 'ok' }] });
    mocks.getSessionToolTimeline.mockResolvedValue({ sessionId: 's1', calls: [{ callId: 1, toolName: 'search', family: 'claude', request: {}, response: null, success: null, startedAtMs: 0, completedAtMs: null, durationMs: null, startedAt: '', completedAt: null, inFlight: true }], count: 1 });
});

describe('mcp store', () => {
    it('loadCatalog fills tools / stats / signals from the API', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        expect(mcp.catalogStatus).toBe('ready');
        expect(mcp.tools.length).toBe(1);
        expect(mcp.signalSource).toBe('cache');
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(1);
    });

    it('loadCatalog is idempotent — a second call hits no endpoints', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadCatalog();
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(1);
    });

    it('loadCatalog(force=true) re-fetches even after ready', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadCatalog(true);
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(2);
    });

    it('insightFor returns the cached insight row by tool name', async () => {
        const mcp = await freshStore();
        await mcp.loadInsights();
        expect(mcp.insightFor('search')?.variant).toBe('win');
        expect(mcp.insightFor('missing')).toBeUndefined();
    });

    it('loadSessionTimeline caches per session id', async () => {
        const mcp = await freshStore();
        await mcp.loadSessionTimeline('s1');
        await mcp.loadSessionTimeline('s1');
        expect(mocks.getSessionToolTimeline).toHaveBeenCalledTimes(1);
        expect(mcp.sessionTimelines['s1'].length).toBe(1);
    });

    it('refresh clears the timeline cache and re-loads catalog + insights', async () => {
        const mcp = await freshStore();
        await mcp.loadCatalog();
        await mcp.loadInsights();
        await mcp.loadSessionTimeline('s1');
        await mcp.refresh();
        expect(mocks.mcpListTools).toHaveBeenCalledTimes(2);
        expect(mocks.getToolInsights).toHaveBeenCalledTimes(2);
        expect(Object.keys(mcp.sessionTimelines)).toHaveLength(0);
    });

    it('marks catalog status error when the API throws', async () => {
        mocks.mcpListTools.mockRejectedValueOnce(new Error('boom'));
        const mcp = await freshStore();
        await mcp.loadCatalog();
        expect(mcp.catalogStatus).toBe('error');
    });
});
