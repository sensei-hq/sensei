import { describe, it, expect } from 'vitest';
import {
    SECTIONS,
    isSectionActive,
    healthRows,
    ftrBand,
    ftrDelta,
    ftrSparkTone,
} from './project-sidebar-view.js';

describe('project-sidebar-view', () => {
    describe('isSectionActive', () => {
        it('matches the current section path', () => {
            expect(isSectionActive('/project/abc/overview', 'abc', 'overview')).toBe(true);
        });
        it('does not match a different section', () => {
            expect(isSectionActive('/project/abc/sessions', 'abc', 'overview')).toBe(false);
        });
        it('stays active on a nested path within the section', () => {
            expect(isSectionActive('/project/abc/sessions/xyz', 'abc', 'sessions')).toBe(true);
        });
        it('scopes to the project id', () => {
            expect(isSectionActive('/project/other/overview', 'abc', 'overview')).toBe(false);
        });
    });

    describe('healthRows', () => {
        it('renders FTR as a whole-number percent and sessions verbatim', () => {
            const [ftr, sessions] = healthRows(0.82, 28);
            expect(ftr).toEqual({ label: 'FTR · 14d', value: '82%' });
            expect(sessions).toEqual({ label: 'Sessions · 7d', value: '28' });
        });
        it('shows the em dash for absent data, never a fabricated 0', () => {
            const [ftr, sessions] = healthRows(null, null);
            expect(ftr.value).toBe('—');
            expect(sessions.value).toBe('—');
        });
        it('renders a genuine zero session count as 0 (not the em dash)', () => {
            expect(healthRows(null, 0)[1].value).toBe('0');
        });
    });

    describe('SECTIONS', () => {
        it('leads with intake then overview and includes about', () => {
            expect(SECTIONS[0].id).toBe('intake');
            expect(SECTIONS.some((s) => s.id === 'overview')).toBe(true);
            expect(SECTIONS.some((s) => s.id === 'about')).toBe(true);
        });
        it('exposes a Metrics section right after Overview', () => {
            const ids = SECTIONS.map((s) => s.id);
            expect(ids).toContain('metrics');
            expect(ids.indexOf('metrics')).toBe(ids.indexOf('overview') + 1);
        });
        it('gives every section a unique id and a kanji glyph', () => {
            const ids = SECTIONS.map((s) => s.id);
            expect(new Set(ids).size).toBe(ids.length);
            expect(SECTIONS.every((s) => s.kanji.length >= 1)).toBe(true);
        });
    });

    describe('ftrBand', () => {
        it('bands FTR by threshold, and null → ink-faint (no fabricated band)', () => {
            expect(ftrBand(null)).toBe('ink-faint');
            expect(ftrBand(0.85)).toBe('success');
            expect(ftrBand(0.7)).toBe('accent');
            expect(ftrBand(0.4)).toBe('warning');
        });
    });

    describe('ftrDelta', () => {
        it('renders the point delta, steady, or an em dash when a window is absent', () => {
            expect(ftrDelta(0.72, 0.6)).toBe('+12 pt');
            expect(ftrDelta(0.6, 0.72)).toBe('−12 pt');
            expect(ftrDelta(0.6, 0.6)).toBe('steady');
            expect(ftrDelta(0.6, null)).toBe('—');
            expect(ftrDelta(null, 0.6)).toBe('—');
        });
    });

    describe('ftrSparkTone', () => {
        it('maps the trend direction to a spark tone (neutral without a prior)', () => {
            expect(ftrSparkTone(0.7, 0.6)).toBe('good');
            expect(ftrSparkTone(0.6, 0.7)).toBe('bad');
            expect(ftrSparkTone(0.6, 0.6)).toBe('neutral');
            expect(ftrSparkTone(0.6, null)).toBe('neutral');
        });
    });
});
