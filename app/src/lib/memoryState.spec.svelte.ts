import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
    memoryState,
    memoryUsageStrip,
    actionErrorMessage,
    isTerminalStatus,
} from './memoryState.svelte.js';
import { mockMemory, mockMemoryDetail } from './setup/mock-contracts.js';

const listMemoriesMock = vi.fn();
const acceptMock       = vi.fn();
const rejectMock       = vi.fn();
const detailMock       = vi.fn();
const reinforceMock    = vi.fn();
const challengeMock    = vi.fn();
const archiveMock      = vi.fn();
const dismissMock      = vi.fn();

vi.mock('./api.js', () => ({
    senseiApi: (_port: number) => ({
        listMemories:    listMemoriesMock,
        acceptProposal:  acceptMock,
        rejectProposal:  rejectMock,
        getMemoryDetail: detailMock,
        reinforceMemory: reinforceMock,
        challengeMemory: challengeMock,
        archiveMemory:   archiveMock,
        dismissMemory:   dismissMock,
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
        reinforceMock.mockReset();
        challengeMock.mockReset();
        archiveMock.mockReset();
        dismissMock.mockReset();
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

describe('memoryState lifecycle actions', () => {
    beforeEach(() => {
        memoryState.triage = [];
        memoryState.active = [];
        memoryState.archive = [];
        listMemoriesMock.mockReset();
        reinforceMock.mockReset();
        challengeMock.mockReset();
        archiveMock.mockReset();
        dismissMock.mockReset();
    });

    it('reinforce calls the api and re-fetches the active list on success', async () => {
        memoryState.active = [mockMemory({ id: 'm-r', status: 'active' })];
        reinforceMock.mockResolvedValue({ ok: true, data: { id: 'm-r', reinforced: true } });
        listMemoriesMock.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'active' ? [mockMemory({ id: 'm-r', status: 'reinforced' })] : [],
        }));
        await memoryState.reinforce('m-r');
        expect(reinforceMock).toHaveBeenCalledWith('m-r');
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.active[0].status).toBe('reinforced');
        expect(memoryState.actionError('m-r')).toBeNull();
        expect(memoryState.isActing('m-r')).toBe(false);
    });

    it('reinforce surfaces an error and leaves the active list untouched on failure', async () => {
        memoryState.active = [mockMemory({ id: 'm-r2', status: 'active' })];
        reinforceMock.mockResolvedValue({ ok: false, error: { status: 500, message: 'boom' } });
        listMemoriesMock.mockResolvedValue({ memories: [] });
        await memoryState.reinforce('m-r2');
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.actionError('m-r2')).toBe("couldn't reinforce this right now. try again.");
        expect(memoryState.isActing('m-r2')).toBe(false);
        expect(listMemoriesMock).not.toHaveBeenCalled();
    });

    it('challenge keeps the memory in the active set and re-fetches on success', async () => {
        memoryState.active = [mockMemory({ id: 'm-c', status: 'active' })];
        challengeMock.mockResolvedValue({ ok: true, data: { id: 'm-c', status: 'challenged' } });
        listMemoriesMock.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'active' ? [mockMemory({ id: 'm-c', status: 'challenged' })] : [],
        }));
        await memoryState.challenge('m-c');
        expect(challengeMock).toHaveBeenCalledWith('m-c');
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.active[0].status).toBe('challenged');
    });

    it('challenge on a terminal memory surfaces the 409 message and stays consistent', async () => {
        memoryState.active = [mockMemory({ id: 'm-c9', status: 'active' })];
        challengeMock.mockResolvedValue({ ok: false, error: { status: 409, message: 'Conflict' } });
        await memoryState.challenge('m-c9');
        // Memory unchanged, no destination re-fetch, error surfaced non-fatally.
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.actionError('m-c9')).toBe('this memory is already archived or rejected.');
        expect(memoryState.isActing('m-c9')).toBe(false);
        expect(listMemoriesMock).not.toHaveBeenCalled();
    });

    it('archive removes from active and refreshes the archive tab on success', async () => {
        memoryState.active = [mockMemory({ id: 'm-a', status: 'active' })];
        archiveMock.mockResolvedValue({ ok: true, data: { id: 'm-a', status: 'archived' } });
        listMemoriesMock.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'archived' ? [mockMemory({ id: 'm-a', status: 'archived' })] : [],
        }));
        await memoryState.archiveMemory('m-a');
        expect(archiveMock).toHaveBeenCalledWith('m-a');
        expect(memoryState.active).toHaveLength(0);
        expect(memoryState.archive).toHaveLength(1);
        expect(memoryState.archive[0].status).toBe('archived');
    });

    it('archive leaves state untouched and surfaces an error on failure', async () => {
        memoryState.active = [mockMemory({ id: 'm-a2', status: 'active' })];
        archiveMock.mockResolvedValue({ ok: false, error: { status: 500, message: 'boom' } });
        await memoryState.archiveMemory('m-a2');
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.archive).toHaveLength(0);
        expect(memoryState.actionError('m-a2')).toBe("couldn't archive this right now. try again.");
    });

    it('dismiss moves the memory out of the active set into archive (rejected) on success', async () => {
        memoryState.active = [mockMemory({ id: 'm-d', status: 'active' })];
        dismissMock.mockResolvedValue({ ok: true, data: { id: 'm-d', status: 'rejected' } });
        listMemoriesMock.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'rejected' ? [mockMemory({ id: 'm-d', status: 'rejected' })] : [],
        }));
        await memoryState.dismiss('m-d');
        expect(dismissMock).toHaveBeenCalledWith('m-d');
        expect(memoryState.active).toHaveLength(0);
        expect(memoryState.archive).toHaveLength(1);
        expect(memoryState.archive[0].status).toBe('rejected');
    });

    it('dismiss on a terminal memory surfaces the 409 message and stays consistent', async () => {
        memoryState.active = [mockMemory({ id: 'm-d9', status: 'active' })];
        dismissMock.mockResolvedValue({ ok: false, error: { status: 409, message: 'Conflict' } });
        await memoryState.dismiss('m-d9');
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.archive).toHaveLength(0);
        expect(memoryState.actionError('m-d9')).toBe('this memory is already archived or rejected.');
        expect(listMemoriesMock).not.toHaveBeenCalled();
    });

    it('guards against a double-submit while an action is in flight', async () => {
        memoryState.active = [mockMemory({ id: 'm-g', status: 'active' })];
        let resolve!: (v: unknown) => void;
        reinforceMock.mockReturnValue(new Promise((r) => { resolve = r; }));
        listMemoriesMock.mockResolvedValue({ memories: [mockMemory({ id: 'm-g', status: 'reinforced' })] });

        const first = memoryState.reinforce('m-g');
        expect(memoryState.isActing('m-g')).toBe(true);
        // Second call while the first is pending must be ignored.
        await memoryState.reinforce('m-g');
        expect(reinforceMock).toHaveBeenCalledTimes(1);

        resolve({ ok: true, data: { id: 'm-g', reinforced: true } });
        await first;
        expect(memoryState.isActing('m-g')).toBe(false);
    });
});

describe('actionErrorMessage', () => {
    it('maps 409 to the terminal-state message', () => {
        expect(actionErrorMessage('challenge', 409)).toBe('this memory is already archived or rejected.');
        expect(actionErrorMessage('dismiss', 409)).toBe('this memory is already archived or rejected.');
    });

    it('maps any other status to a transient per-verb try-again message', () => {
        expect(actionErrorMessage('reinforce', 500)).toBe("couldn't reinforce this right now. try again.");
        expect(actionErrorMessage('archive', 0)).toBe("couldn't archive this right now. try again.");
    });
});

describe('isTerminalStatus', () => {
    it('treats archived and rejected as terminal', () => {
        expect(isTerminalStatus('archived')).toBe(true);
        expect(isTerminalStatus('rejected')).toBe(true);
    });

    it('treats active-set statuses as non-terminal', () => {
        expect(isTerminalStatus('active')).toBe(false);
        expect(isTerminalStatus('reinforced')).toBe(false);
        expect(isTerminalStatus('challenged')).toBe(false);
        expect(isTerminalStatus('battle_tested')).toBe(false);
        expect(isTerminalStatus('proposed')).toBe(false);
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
