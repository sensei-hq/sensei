/**
 * Unit tests for the Diagnostic Logs page helper functions.
 * Pure logic — no Svelte rendering needed.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
    fmtMs, outcomeColor, anonymize, formatTime, dateKey, timeOfDay,
    sessionSummary, groupByDate,
    moduleLabel, moduleKanji,
    isBootstrapTrace, actionMeta,
    buildTitle, buildBody,
    ACTION_META,
} from './helpers.js';
import type { LogSession, BootstrapTrace, LogEntry } from '$lib/types.js';

// ── Fixtures ──────────────────────────────────────────────────────────────

function makeTrace(overrides: Partial<BootstrapTrace> = {}): BootstrapTrace {
    return {
        id:            'trace-1',
        ts:            '2024-01-15T10:00:00Z',
        action_type:   'check',
        step:          'homebrew',
        desc:          'Homebrew package manager',
        cmd:           'brew --version',
        exit:          0,
        out:           'Homebrew 4.2.0',
        err:           '',
        ms:            45,
        ok:            true,
        fix_attempted: false,
        fix_approach:  null,
        fix_ok:        null,
        ...overrides,
    };
}

function makeSession(overrides: Partial<LogSession> = {}): LogSession {
    return {
        id:          'sess-1',
        module:      'bootstrap',
        started_at:  '2024-01-15T10:00:00Z',
        app_version: '1.2.3',
        system_info: {
            os:        'macOS 14.2 (/Users/jerry/home)',
            arch:      'aarch64',
            ram_gb:    16,
            cpu_cores: 8,
        },
        outcome:     'success',
        duration_ms: 3200,
        traces:      [makeTrace()],
        ...overrides,
    };
}

// ── fmtMs ─────────────────────────────────────────────────────────────────

describe('fmtMs', () => {
    it('shows ms for values under 1000', () => {
        expect(fmtMs(0)).toBe('0ms');
        expect(fmtMs(45)).toBe('45ms');
        expect(fmtMs(999)).toBe('999ms');
    });

    it('shows seconds with one decimal for 1000ms+', () => {
        expect(fmtMs(1000)).toBe('1.0s');
        expect(fmtMs(1500)).toBe('1.5s');
        expect(fmtMs(3200)).toBe('3.2s');
    });
});

// ── outcomeColor ──────────────────────────────────────────────────────────

describe('outcomeColor', () => {
    it('maps success to jade',   () => expect(outcomeColor('success')).toBe('oklch(var(--color-success-z5) / 1)'));
    it('maps partial to amber',  () => expect(outcomeColor('partial')).toBe('oklch(var(--color-warning-z5) / 1)'));
    it('maps failed to shu',     () => expect(outcomeColor('failed')).toBe('oklch(var(--color-primary-z5) / 1)'));
    it('maps unknown to shu',    () => expect(outcomeColor('unknown')).toBe('oklch(var(--color-primary-z5) / 1)'));
});

// ── anonymize ─────────────────────────────────────────────────────────────

describe('anonymize', () => {
    it('replaces /Users/name/ with ~/', () => {
        expect(anonymize('/Users/jerry/projects/foo')).toBe('~/projects/foo');
    });
    it('replaces any username', () => {
        expect(anonymize('/Users/alice/dev/bar')).toBe('~/dev/bar');
    });
    it('leaves strings without /Users/ untouched', () => {
        expect(anonymize('brew --version')).toBe('brew --version');
    });
    it('replaces multiple occurrences', () => {
        expect(anonymize('/Users/bob/a and /Users/bob/b')).toBe('~/a and ~/b');
    });
});

// ── formatTime / dateKey / timeOfDay ─────────────────────────────────────

describe('formatTime', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2024-01-15T12:00:00'));
    });
    afterEach(() => vi.useRealTimers());

    it('shows "Today" for same-day',    () => expect(formatTime('2024-01-15T08:30:00')).toMatch(/^Today · /));
    it('shows "Yesterday" for prior',   () => expect(formatTime('2024-01-14T09:00:00')).toMatch(/^Yesterday · /));
    it('shows month/day for older',     () => expect(formatTime('2024-01-10T09:00:00')).toMatch(/^Jan 10 · /));
});

describe('dateKey', () => {
    beforeEach(() => { vi.useFakeTimers(); vi.setSystemTime(new Date('2024-01-15T12:00:00')); });
    afterEach(() => vi.useRealTimers());

    it('returns date portion only', () => expect(dateKey('2024-01-15T08:30:00')).toBe('Today'));
    it('returns Yesterday',         () => expect(dateKey('2024-01-14T09:00:00')).toBe('Yesterday'));
});

describe('timeOfDay', () => {
    beforeEach(() => { vi.useFakeTimers(); vi.setSystemTime(new Date('2024-01-15T12:00:00')); });
    afterEach(() => vi.useRealTimers());

    it('returns time portion only', () => {
        const t = timeOfDay('2024-01-15T08:30:00');
        expect(t).toMatch(/^\d{2}:\d{2}$/);
    });
});

// ── moduleLabel / moduleKanji ─────────────────────────────────────────────

describe('moduleLabel', () => {
    it('bootstrap → Bootstrap',    () => expect(moduleLabel('bootstrap')).toBe('Bootstrap'));
    it('wizard → Setup wizard',    () => expect(moduleLabel('wizard')).toBe('Setup wizard'));
    it('projects → Projects',      () => expect(moduleLabel('projects')).toBe('Projects'));
    it('unknown → falls back',     () => expect(moduleLabel('other')).toBe('other'));
});

describe('moduleKanji', () => {
    it('bootstrap → 健', () => expect(moduleKanji('bootstrap')).toBe('健'));
    it('wizard → 導',    () => expect(moduleKanji('wizard')).toBe('導'));
    it('unknown → ◆',   () => expect(moduleKanji('unknown')).toBe('◆'));
});

// ── sessionSummary ────────────────────────────────────────────────────────

describe('sessionSummary', () => {
    it('shows duration and step count', () => {
        const s = makeSession({ duration_ms: 3200, traces: [makeTrace()] });
        expect(sessionSummary(s)).toBe('3.2s · 1 steps');
    });

    it('appends fix count when > 0', () => {
        const s = makeSession({
            duration_ms: 1000,
            traces: [makeTrace({ fix_attempted: true }), makeTrace({ fix_attempted: true })],
        });
        expect(sessionSummary(s)).toContain('2 fixes');
    });

    it('uses singular "fix" for 1 fix', () => {
        const s = makeSession({
            duration_ms: 1000,
            traces: [makeTrace({ fix_attempted: true })],
        });
        expect(sessionSummary(s)).toContain('1 fix');
        expect(sessionSummary(s)).not.toContain('1 fixes');
    });

    it('omits fix count when no fixes', () => {
        const s = makeSession({ traces: [makeTrace({ fix_attempted: false })] });
        expect(sessionSummary(s)).not.toContain('fix');
    });
});

// ── groupByDate ───────────────────────────────────────────────────────────

describe('groupByDate', () => {
    beforeEach(() => { vi.useFakeTimers(); vi.setSystemTime(new Date('2024-01-15T12:00:00')); });
    afterEach(() => vi.useRealTimers());

    it('groups sessions by date, newest date first', () => {
        const sessions = [
            makeSession({ id: 's1', started_at: '2024-01-15T10:00:00' }),
            makeSession({ id: 's2', started_at: '2024-01-14T09:00:00' }),
        ];
        const groups = groupByDate(sessions);
        expect(groups).toHaveLength(2);
        expect(groups[0].date).toBe('Today');
        expect(groups[1].date).toBe('Yesterday');
    });

    it('sessions of different modules appear in same flat list under date', () => {
        const sessions = [
            makeSession({ id: 's1', started_at: '2024-01-15T10:00:00', module: 'bootstrap' }),
            makeSession({ id: 's2', started_at: '2024-01-15T10:05:00', module: 'wizard' }),
        ];
        const groups = groupByDate(sessions);
        expect(groups).toHaveLength(1);
        expect(groups[0].sessions).toHaveLength(2);
    });

    it('sorts sessions reverse-chronologically within a date', () => {
        const sessions = [
            makeSession({ id: 's1', started_at: '2024-01-15T08:00:00' }),
            makeSession({ id: 's2', started_at: '2024-01-15T11:00:00' }),
        ];
        const groups = groupByDate(sessions);
        expect(groups[0].sessions[0].id).toBe('s2'); // 11:00 first
        expect(groups[0].sessions[1].id).toBe('s1'); // 08:00 second
    });

    it('returns empty array for no sessions', () => {
        expect(groupByDate([])).toEqual([]);
    });
});

// ── isBootstrapTrace ──────────────────────────────────────────────────────

describe('isBootstrapTrace', () => {
    it('returns true for BootstrapTrace', () => expect(isBootstrapTrace(makeTrace())).toBe(true));
    it('returns false for LogEntry', () => {
        const e: LogEntry = { id: 'e1', ts: '', level: 'info', layer: 'ui', step: 's', msg: 'm' };
        expect(isBootstrapTrace(e)).toBe(false);
    });
    it('returns false for null', () => expect(isBootstrapTrace(null)).toBe(false));
});

// ── actionMeta ────────────────────────────────────────────────────────────

describe('actionMeta', () => {
    it('check → CHECK',       () => expect(actionMeta('check').label).toBe('CHECK'));
    it('resolve → RESOLVE',   () => expect(actionMeta('resolve').label).toBe('RESOLVE'));
    it('instruct → INSTRUCT', () => expect(actionMeta('instruct').label).toBe('INSTRUCT'));
    it('unknown → CHECK',     () => expect(actionMeta('unknown').label).toBe('CHECK'));
});

// ── buildTitle ────────────────────────────────────────────────────────────

describe('buildTitle', () => {
    it('includes arch and version', () => {
        const title = buildTitle(makeSession());
        expect(title).toContain('aarch64');
        expect(title).toContain('v1.2.3');
    });
    it('anonymizes OS path', () => {
        const title = buildTitle(makeSession());
        expect(title).not.toContain('/Users/jerry/');
    });
});

// ── buildBody ─────────────────────────────────────────────────────────────

describe('buildBody', () => {
    it('includes system info section',    () => expect(buildBody(makeSession(), '')).toContain('## System info'));
    it('includes bootstrap trace table',  () => expect(buildBody(makeSession(), '')).toContain('## Bootstrap trace'));
    it('anonymizes paths',                () => expect(buildBody(makeSession(), '')).not.toContain('/Users/jerry/'));

    it('appends additional context when provided', () => {
        const body = buildBody(makeSession(), 'It crashed');
        expect(body).toContain('## Additional context');
        expect(body).toContain('It crashed');
    });
    it('omits additional context when empty', () => {
        expect(buildBody(makeSession(), '')).not.toContain('## Additional context');
    });

    it('marks successful trace with ✓',   () => expect(buildBody(makeSession({ traces: [makeTrace({ ok: true })] }), '')).toContain('| ✓ |'));
    it('marks failed trace with ✗',       () => expect(buildBody(makeSession({ traces: [makeTrace({ ok: false, fix_ok: false })] }), '')).toContain('| ✗ |'));
    it('marks auto-fixed trace as fixed', () => expect(buildBody(makeSession({ traces: [makeTrace({ ok: false, fix_ok: true })] }), '')).toContain('| ✓ fixed |'));

    it('ignores non-bootstrap LogEntry traces', () => {
        const e: LogEntry = { id: 'e1', ts: '', level: 'info', layer: 'ui', step: 'prefs_load', msg: 'ok' };
        const body = buildBody(makeSession({ traces: [e] }), '');
        expect(body).not.toContain('prefs_load');
    });
});
