/**
 * Tests for LogsPageState — reactive state for the Diagnostic Logs page.
 * Uses $state/$derived so this is .spec.svelte.ts.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { LogsPageState } from './state.svelte.js';
import type { LogSession } from '$lib/types.js';

// ── Fixtures ──────────────────────────────────────────────────────────────

function makeSession(id: string, startedAt: string, outcome: LogSession['outcome'] = 'success'): LogSession {
    return {
        id,
        module:      'bootstrap',
        started_at:  startedAt,
        app_version: '1.0.0',
        system_info: { os: 'macOS', arch: 'aarch64', ram_gb: 16, cpu_cores: 8 },
        outcome,
        duration_ms: 1000,
        traces:      [],
    };
}

// Pin "now" so dateKey() returns predictable values
beforeEach(() => { vi.useFakeTimers(); vi.setSystemTime(new Date('2024-01-15T12:00:00')); });
afterEach(() => vi.useRealTimers());

// ── Defaults ──────────────────────────────────────────────────────────────

describe('LogsPageState defaults (no sessions)', () => {
    it('selectedId is null',       () => expect(new LogsPageState().selectedId).toBeNull());
    it('session is null',          () => expect(new LogsPageState().session).toBeNull());
    it('modal is closed',          () => expect(new LogsPageState().showModal).toBe(false));
    it('addCtx is empty',          () => expect(new LogsPageState().addCtx).toBe(''));
    it('copied is false',          () => expect(new LogsPageState().copied).toBe(false));
    it('expandedTraceId is null',  () => expect(new LogsPageState().expandedTraceId).toBeNull());
    it('expandedDates is empty',   () => expect(new LogsPageState().expandedDates.size).toBe(0));
});

// ── Initialisation with sessions ──────────────────────────────────────────

describe('LogsPageState init with sessions (constructor)', () => {
    const sessions = [
        makeSession('s1', '2024-01-15T10:00:00'),
        makeSession('s2', '2024-01-14T09:00:00'),
    ];

    it('selects first session by default', () => {
        const s = new LogsPageState(sessions);
        expect(s.selectedId).toBe('s1');
    });

    it('auto-expands the date group of the first session', () => {
        const s = new LogsPageState(sessions);
        expect(s.expandedDates.has('Today')).toBe(true);
    });

    it('session derived resolves to first session', () => {
        const s = new LogsPageState(sessions);
        expect(s.session?.id).toBe('s1');
    });
});

describe('setSessions', () => {
    it('selects first session when none selected yet', () => {
        const s = new LogsPageState();
        s.setSessions([makeSession('s1', '2024-01-15T10:00:00')]);
        expect(s.selectedId).toBe('s1');
    });

    it('auto-expands the date group', () => {
        const s = new LogsPageState();
        s.setSessions([makeSession('s1', '2024-01-15T10:00:00')]);
        expect(s.expandedDates.has('Today')).toBe(true);
    });

    it('does not change selection if already set', () => {
        const sessions = [makeSession('s1', '2024-01-15T10:00:00'), makeSession('s2', '2024-01-14T09:00:00')];
        const s = new LogsPageState(sessions);
        s.selectSession('s2');
        s.setSessions(sessions);
        expect(s.selectedId).toBe('s2'); // unchanged
    });
});

// ── selectSession ─────────────────────────────────────────────────────────

describe('selectSession', () => {
    it('updates selectedId', () => {
        const sessions = [makeSession('s1', '2024-01-15T10:00:00'), makeSession('s2', '2024-01-14T09:00:00')];
        const s = new LogsPageState(sessions);
        s.selectSession('s2');
        expect(s.selectedId).toBe('s2');
        expect(s.session?.id).toBe('s2');
    });

    it('resets expandedTraceId on session change', () => {
        const sessions = [makeSession('s1', '2024-01-15T10:00:00'), makeSession('s2', '2024-01-14T09:00:00')];
        const s = new LogsPageState(sessions);
        s.toggleTrace('trace-1');
        expect(s.expandedTraceId).toBe('trace-1');
        s.selectSession('s2');
        expect(s.expandedTraceId).toBeNull();
    });
});

// ── toggleDate ────────────────────────────────────────────────────────────

describe('toggleDate', () => {
    it('adds date on first call', () => {
        const s = new LogsPageState();
        s.toggleDate('Yesterday');
        expect(s.expandedDates.has('Yesterday')).toBe(true);
    });

    it('removes date on second call', () => {
        const s = new LogsPageState();
        s.toggleDate('Yesterday');
        s.toggleDate('Yesterday');
        expect(s.expandedDates.has('Yesterday')).toBe(false);
    });

    it('multiple dates can be open simultaneously', () => {
        const s = new LogsPageState();
        s.toggleDate('Today');
        s.toggleDate('Yesterday');
        expect(s.expandedDates.has('Today')).toBe(true);
        expect(s.expandedDates.has('Yesterday')).toBe(true);
    });
});

// ── toggleTrace (accordion) ───────────────────────────────────────────────

describe('toggleTrace', () => {
    it('opens a trace', () => {
        const s = new LogsPageState();
        s.toggleTrace('trace-1');
        expect(s.expandedTraceId).toBe('trace-1');
    });

    it('closes the same trace on second call', () => {
        const s = new LogsPageState();
        s.toggleTrace('trace-1');
        s.toggleTrace('trace-1');
        expect(s.expandedTraceId).toBeNull();
    });

    it('accordion: opening a second trace closes the first', () => {
        const s = new LogsPageState();
        s.toggleTrace('trace-1');
        s.toggleTrace('trace-2');
        expect(s.expandedTraceId).toBe('trace-2');
    });
});

// ── modal ─────────────────────────────────────────────────────────────────

describe('modal state', () => {
    it('openModal sets showModal true',   () => { const s = new LogsPageState(); s.openModal();  expect(s.showModal).toBe(true); });
    it('closeModal sets showModal false', () => { const s = new LogsPageState(); s.openModal(); s.closeModal(); expect(s.showModal).toBe(false); });
});

// ── setCopied ─────────────────────────────────────────────────────────────

describe('setCopied', () => {
    it('sets true',  () => { const s = new LogsPageState(); s.setCopied(true);  expect(s.copied).toBe(true); });
    it('sets false', () => { const s = new LogsPageState(); s.setCopied(true); s.setCopied(false); expect(s.copied).toBe(false); });
});
