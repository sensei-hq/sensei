import { describe, it, expect, vi, beforeEach } from 'vitest';
import { memoryState, memoryUsageStrip } from './memoryState.svelte.js';
import { mockMemory, mockMemoryDetail } from './setup/mock-contracts.js';

const listMemoriesMock = vi.fn();
const acceptMock       = vi.fn();
const rejectMock       = vi.fn();
const detailMock       = vi.fn();

vi.mock('./api.js', () => ({
    senseiApi: (_port: number) => ({
        listMemories:    listMemoriesMock,
        acceptProposal:  acceptMock,
        rejectProposal:  rejectMock,
        getMemoryDetail: detailMock,
    }),
}));

vi.mock('./appstate.svelte.js', () => ({
    appState: { port: 7745 },
}));

describe('memoryState', () => {
    beforeEach(() => {
        memoryState.triage = [];
        memoryState.active = [];
        memoryState.archive = [];
        memoryState.detail = null;
        memoryState.selected = null;
        listMemoriesMock.mockReset();
        acceptMock.mockReset();
        rejectMock.mockReset();
        detailMock.mockReset();
    });

    it('partitions memories by tab on load', async () => {
        listMemoriesMock.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'proposed' ? [mockMemory({ id: 'p1', status: 'proposed' })]
                    : status === 'active'   ? [mockMemory({ id: 'a1', status: 'active' })]
                    : status === 'archived' ? [mockMemory({ id: 'x1', status: 'archived' })]
                    : []
        }));
        await memoryState.load('proj-1');
        expect(memoryState.triage).toHaveLength(1);
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.archive).toHaveLength(1);
    });

    it('accept removes from triage on success', async () => {
        memoryState.triage = [mockMemory({ id: 'm-acc', status: 'proposed' })];
        acceptMock.mockResolvedValue({ ok: true, data: { id: 'm-acc', status: 'active' } });
        listMemoriesMock.mockResolvedValue({ memories: [] });
        await memoryState.accept('m-acc');
        expect(memoryState.triage).toHaveLength(0);
        expect(acceptMock).toHaveBeenCalledWith('m-acc');
    });

    it('accept does NOT mutate when api call fails', async () => {
        memoryState.triage = [mockMemory({ id: 'm-fail', status: 'proposed' })];
        acceptMock.mockResolvedValue({ ok: false, error: { status: 500, message: 'oops' } });
        listMemoriesMock.mockResolvedValue({ memories: [] });
        await memoryState.accept('m-fail');
        expect(memoryState.triage).toHaveLength(1);
    });

    it('reject removes from triage and refreshes archive', async () => {
        memoryState.triage = [mockMemory({ id: 'm-rej', status: 'proposed' })];
        rejectMock.mockResolvedValue({ ok: true, data: { id: 'm-rej', status: 'rejected' } });
        // fetchTab('archive') calls listMemories for both 'archived' and 'rejected' statuses.
        // Only return the memory for 'rejected' so archive ends up with exactly 1 item.
        listMemoriesMock.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'rejected' ? [mockMemory({ id: 'm-rej', status: 'rejected' })] : [],
        }));
        await memoryState.reject('m-rej');
        expect(memoryState.triage).toHaveLength(0);
        expect(rejectMock).toHaveBeenCalledWith('m-rej');
        expect(memoryState.archive).toHaveLength(1);
    });

    it('select fetches detail when api succeeds', async () => {
        detailMock.mockResolvedValue({ ok: true, data: mockMemoryDetail() });
        await memoryState.select('m-1');
        expect(memoryState.selected).toBe('m-1');
        expect(memoryState.detail).not.toBeNull();
    });
});

describe('memoryUsageStrip', () => {
    it('formats the three 7-day counters with the last-7-days window', () => {
        const s = memoryUsageStrip({ loaded_last_7d: 5, followed_last_7d: 3, skipped_last_7d: 1 });
        expect(s.loaded).toBe('loaded 5 times');
        expect(s.followed).toBe('followed 3');
        expect(s.skipped).toBe('skipped 1');
        expect(s.window).toBe('in the last 7 days');
    });

    it('renders genuine zeros, never blank', () => {
        const s = memoryUsageStrip({ loaded_last_7d: 0, followed_last_7d: 0, skipped_last_7d: 0 });
        expect(s.loaded).toBe('loaded 0 times');
        expect(s.followed).toBe('followed 0');
        expect(s.skipped).toBe('skipped 0');
    });

    it('pluralizes a single load as "time"', () => {
        expect(memoryUsageStrip({ loaded_last_7d: 1, followed_last_7d: 0, skipped_last_7d: 0 }).loaded)
            .toBe('loaded 1 time');
    });
});
