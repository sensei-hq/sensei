// @vitest-environment jsdom
/**
 * Tests for ProjectMemoriesState — the Project window · Memories ready-to-share
 * derivations, chip semantics, and the generalise / widen state machines.
 * Uses $state/$derived so this is a .spec.svelte.ts.
 */
import { describe, it, expect, vi } from 'vitest';
import { ProjectMemoriesState, SCOPE_LADDER, type MemoriesApi } from './memories-state.svelte.js';
import type { ProjectMemory } from '$lib/types.js';

// ── Fixtures ──────────────────────────────────────────────────────────────

function mem(id: string, over: Partial<ProjectMemory> = {}): ProjectMemory {
    return {
        id,
        name: id,
        title: `memory ${id}`,
        kind: 'convention',
        type: 'convention',
        strength: 1,
        scope: 'project',
        generalised: false,
        generalisedContent: null,
        ...over,
    };
}

// ── Defaults (no memories) ──────────────────────────────────────────────────

describe('ProjectMemoriesState defaults', () => {
    it('readyCount is 0', () => expect(new ProjectMemoriesState().readyCount).toBe(0));
    it('formingCount is 0', () => expect(new ProjectMemoriesState().formingCount).toBe(0));
    it('hasReady is false', () => expect(new ProjectMemoriesState().hasReady).toBe(false));
    it('countLabel reflects empty', () =>
        expect(new ProjectMemoriesState().countLabel).toBe('0 active · 0 generalised'));
    it('widen submenu starts closed', () => {
        const s = new ProjectMemoriesState();
        expect(s.widenOpenId).toBeNull();
        expect(s.pendingScope).toBeNull();
    });
});

// ── Counts ──────────────────────────────────────────────────────────────────

describe('ready / forming counts', () => {
    const s = new ProjectMemoriesState([
        mem('a', { generalised: true }),
        mem('b', { generalised: true }),
        mem('c', { generalised: false }),
    ]);

    it('readyCount counts only generalised === true', () => expect(s.readyCount).toBe(2));
    it('formingCount is the remainder', () => expect(s.formingCount).toBe(1));
    it('hasReady is true when any are generalised', () => expect(s.hasReady).toBe(true));
    it('countLabel shows active · generalised', () =>
        expect(s.countLabel).toBe('3 active · 2 generalised'));

    it('ready set excludes non-generalised', () => {
        expect(s.ready.map((m) => m.id).sort()).toEqual(['a', 'b']);
    });

    it('recomputes after setMemories (reactive)', () => {
        const t = new ProjectMemoriesState();
        expect(t.readyCount).toBe(0);
        t.setMemories([mem('x', { generalised: true })]);
        expect(t.readyCount).toBe(1);
        expect(t.hasReady).toBe(true);
    });
});

// ── Hero copy ───────────────────────────────────────────────────────────────

describe('heroDisplay copy', () => {
    it('singular headline + eyebrow for one ready', () => {
        const s = new ProjectMemoriesState([mem('a', { generalised: true })]);
        expect(s.heroDisplay.eyebrow).toBe('1 ready to share');
        expect(s.heroDisplay.headline).toBe('one project memory generalised cleanly.');
        expect(s.heroDisplay.action).toBe('review next batch');
    });

    it('plural headline for multiple ready', () => {
        const s = new ProjectMemoriesState([
            mem('a', { generalised: true }),
            mem('b', { generalised: true }),
        ]);
        expect(s.heroDisplay.headline).toBe('2 project memories generalised cleanly.');
    });

    it('meta reports still-forming count', () => {
        const s = new ProjectMemoriesState([
            mem('a', { generalised: true }),
            mem('b', { generalised: false }),
            mem('c', { generalised: false }),
        ]);
        expect(s.heroDisplay.meta).toBe('2 still forming');
    });

    it('meta reports all-generalised when nothing is forming', () => {
        const s = new ProjectMemoriesState([mem('a', { generalised: true })]);
        expect(s.heroDisplay.meta).toBe('all active memories generalised');
    });

    it('copy stays quiet — no exclamation marks', () => {
        const s = new ProjectMemoriesState([mem('a', { generalised: true })]);
        const d = s.heroDisplay;
        for (const line of [d.eyebrow, d.headline, d.body, d.action, d.meta]) {
            expect(line).not.toContain('!');
        }
    });
});

// ── Generalised chip ────────────────────────────────────────────────────────

describe('chipFor', () => {
    const s = new ProjectMemoriesState();

    it('success tone + yes label when generalised', () => {
        const c = s.chipFor(mem('a', { generalised: true }));
        expect(c).toEqual({ label: 'generalised: yes', tone: 'success', done: true });
    });

    it('neutral tone + no label when not generalised', () => {
        const c = s.chipFor(mem('a', { generalised: false }));
        expect(c).toEqual({ label: 'generalised: no', tone: 'neutral', done: false });
    });
});

// ── Generalise state machine ────────────────────────────────────────────────

describe('generalise state machine', () => {
    it('starts idle', () => {
        const s = new ProjectMemoriesState();
        expect(s.isGeneralising('a')).toBe(false);
        expect(s.generaliseError('a')).toBeNull();
    });

    it('startGeneralise sets loading and clears any prior error', () => {
        const s = new ProjectMemoriesState();
        s.failGeneralise('a', 'earlier failure');
        s.startGeneralise('a');
        expect(s.isGeneralising('a')).toBe(true);
        expect(s.generaliseError('a')).toBeNull();
    });

    it('failGeneralise surfaces the message and drops loading', () => {
        const s = new ProjectMemoriesState();
        s.startGeneralise('a');
        s.failGeneralise('a', 'model unavailable');
        expect(s.isGeneralising('a')).toBe(false);
        expect(s.generaliseError('a')).toBe('model unavailable');
    });

    it('finishGeneralise clears loading and error', () => {
        const s = new ProjectMemoriesState();
        s.startGeneralise('a');
        s.finishGeneralise('a');
        expect(s.isGeneralising('a')).toBe(false);
        expect(s.generaliseError('a')).toBeNull();
    });

    it('tracks ids independently', () => {
        const s = new ProjectMemoriesState();
        s.startGeneralise('a');
        expect(s.isGeneralising('a')).toBe(true);
        expect(s.isGeneralising('b')).toBe(false);
    });
});

// ── Widen eligibility ───────────────────────────────────────────────────────

describe('canWiden', () => {
    const s = new ProjectMemoriesState();
    it('true for generalised project-scope memory', () =>
        expect(s.canWiden(mem('a', { generalised: true, scope: 'project' }))).toBe(true));
    it('false when not generalised', () =>
        expect(s.canWiden(mem('a', { generalised: false, scope: 'project' }))).toBe(false));
    it('false when already beyond project scope', () =>
        expect(s.canWiden(mem('a', { generalised: true, scope: 'user' }))).toBe(false));
});

// ── Widen submenu + governance gating ───────────────────────────────────────

describe('widen submenu', () => {
    it('toggles open and closed for a row', () => {
        const s = new ProjectMemoriesState();
        s.toggleWiden('a');
        expect(s.isWidenOpen('a')).toBe(true);
        s.toggleWiden('a');
        expect(s.isWidenOpen('a')).toBe(false);
    });

    it('opening another row resets pending scope', () => {
        const s = new ProjectMemoriesState();
        s.toggleWiden('a');
        s.pendingScope = 'org';
        s.toggleWiden('b');
        expect(s.isWidenOpen('b')).toBe(true);
        expect(s.pendingScope).toBeNull();
    });

    it('non-governed rung proceeds immediately, no pending scope', () => {
        const s = new ProjectMemoriesState();
        const user = SCOPE_LADDER.find((r) => r.scope === 'user')!;
        expect(s.chooseScope(user)).toBe('proceed');
        expect(s.pendingScope).toBeNull();
    });

    it('governed rung requires confirmation and stages the scope', () => {
        const s = new ProjectMemoriesState();
        const org = SCOPE_LADDER.find((r) => r.scope === 'org')!;
        expect(s.chooseScope(org)).toBe('confirm');
        expect(s.pendingScope).toBe('org');
    });

    it('governanceNote names the target scope', () => {
        const s = new ProjectMemoriesState();
        expect(s.governanceNote('collective')).toContain('collective');
        expect(s.governanceNote('collective')).toContain('triage');
    });
});

// ── Promote state machine ───────────────────────────────────────────────────

describe('promote state machine', () => {
    it('startPromote sets loading and clears prior error', () => {
        const s = new ProjectMemoriesState();
        s.failPromote('a', 'earlier');
        s.startPromote('a');
        expect(s.isPromoting('a')).toBe(true);
        expect(s.promoteError('a')).toBeNull();
    });

    it('failPromote surfaces the message and drops loading', () => {
        const s = new ProjectMemoriesState();
        s.startPromote('a');
        s.failPromote('a', 'promote failed');
        expect(s.isPromoting('a')).toBe(false);
        expect(s.promoteError('a')).toBe('promote failed');
    });

    it('finishPromote drops loading and closes the submenu', () => {
        const s = new ProjectMemoriesState();
        s.toggleWiden('a');
        s.startPromote('a');
        s.finishPromote('a');
        expect(s.isPromoting('a')).toBe(false);
        expect(s.isWidenOpen('a')).toBe(false);
        expect(s.pendingScope).toBeNull();
    });
});

// ── Scope ladder shape ──────────────────────────────────────────────────────

describe('SCOPE_LADDER', () => {
    it('has user (ungoverned) then org + global (governed)', () => {
        expect(SCOPE_LADDER.map((r) => r.scope)).toEqual(['user', 'org', 'global']);
        expect(SCOPE_LADDER.find((r) => r.scope === 'user')!.governed).toBe(false);
        expect(SCOPE_LADDER.find((r) => r.scope === 'org')!.governed).toBe(true);
        expect(SCOPE_LADDER.find((r) => r.scope === 'global')!.governed).toBe(true);
    });
});

// ── Actions (#293: moved out of +page.svelte) ─────────────────────────────

const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });

function api(over: Partial<MemoriesApi> = {}): MemoriesApi {
    return {
        tryCreateMemoryShareBatch: vi.fn().mockResolvedValue(ok({ id: 'b1' })),
        tryDecideMemoryShareBatch: vi.fn().mockResolvedValue(ok(undefined)),
        generaliseMemory: vi.fn().mockResolvedValue(ok({ id: 'm1', original: 'o', generalised: 'g', example: null })),
        promoteMemory: vi.fn().mockResolvedValue(ok({ id: 'm1x', status: 'proposed', origin: 'm1' })),
        ...over,
    } as MemoriesApi;
}

describe('ProjectMemoriesState — the share batch', () => {
    it('stages every ready memory for review', () => {
        const s = new ProjectMemoriesState([mem('a', { generalised: true }), mem('b'), mem('c', { generalised: true })]);
        s.toggleSelected('b');
        s.reviewNextBatch();
        expect([...s.selected].sort()).toEqual(['a', 'c']);
    });

    it('proposes the selection, clears it and reloads', async () => {
        const s = new ProjectMemoriesState([mem('a')]);
        s.toggleSelected('a');
        const a = api();
        const reload = vi.fn().mockResolvedValue(undefined);
        await s.proposeBatch(a, 'p1', reload);
        expect(a.tryCreateMemoryShareBatch).toHaveBeenCalledWith('p1', ['a']);
        expect(s.selected.size).toBe(0);
        expect(reload).toHaveBeenCalledOnce();
        expect(s.batchError).toBeNull();
        expect(s.creating).toBe(false);
    });

    it('keeps the selection and says why when the propose fails', async () => {
        const s = new ProjectMemoriesState([mem('a')]);
        s.toggleSelected('a');
        const reload = vi.fn();
        await s.proposeBatch(api({ tryCreateMemoryShareBatch: vi.fn().mockResolvedValue(fail(500, 'boom')) }), 'p1', reload);
        expect([...s.selected]).toEqual(['a']);
        expect(s.batchError).toContain('boom');
        expect(reload).not.toHaveBeenCalled();
        expect(s.creating).toBe(false);
    });

    it('decides a batch and reloads; a refusal is shown on that batch', async () => {
        const s = new ProjectMemoriesState();
        const reload = vi.fn().mockResolvedValue(undefined);
        await s.decide(api(), 'p1', 'b1', 'approved', reload);
        expect(reload).toHaveBeenCalledOnce();
        expect(s.decideError('b1')).toBeNull();

        await s.decide(api({ tryDecideMemoryShareBatch: vi.fn().mockResolvedValue(fail(409, 'Conflict')) }), 'p1', 'b2', 'rejected', reload);
        expect(s.decideError('b2')).toContain('Conflict');
        expect(s.isDeciding('b2')).toBe(false);
        expect(reload).toHaveBeenCalledOnce();
    });
});

describe('ProjectMemoriesState — generalise and widen', () => {
    it('generalises and reloads; a 503 keeps the original and says the model was unavailable', async () => {
        const s = new ProjectMemoriesState([mem('m1')]);
        const reload = vi.fn().mockResolvedValue(undefined);
        await s.generalise(api(), 'm1', reload);
        expect(reload).toHaveBeenCalledOnce();
        expect(s.isGeneralising('m1')).toBe(false);

        await s.generalise(api({ generaliseMemory: vi.fn().mockResolvedValue(fail(503, 'x')) }), 'm1', reload);
        expect(s.generaliseError('m1')).toContain('model was unavailable');
    });

    it('widens to the user’s own scope at once, and a governed scope only after confirming', async () => {
        const s = new ProjectMemoriesState([mem('m1', { generalised: true })]);
        const a = api();
        const reload = vi.fn().mockResolvedValue(undefined);
        const user = SCOPE_LADDER.find((r) => !r.governed)!;
        const org = SCOPE_LADDER.find((r) => r.governed)!;

        await s.pickScope(a, 'm1', user, reload);
        expect(a.promoteMemory).toHaveBeenCalledWith('m1', { gov_scope: user.scope });

        await s.pickScope(a, 'm1', org, reload);
        expect(a.promoteMemory).toHaveBeenCalledTimes(1);
        await s.confirmWiden(a, 'm1', reload);
        expect(a.promoteMemory).toHaveBeenLastCalledWith('m1', { gov_scope: org.scope });
    });

    it('a widen the daemon refuses says the memory is not promotable yet', async () => {
        const s = new ProjectMemoriesState([mem('m1', { generalised: true })]);
        await s.pickScope(api({ promoteMemory: vi.fn().mockResolvedValue(fail(409, 'x')) }), 'm1', SCOPE_LADDER.find((r) => !r.governed)!, vi.fn());
        expect(s.promoteError('m1')).toContain("isn't promotable yet");
    });
});
