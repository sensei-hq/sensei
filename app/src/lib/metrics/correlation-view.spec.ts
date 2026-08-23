import { describe, it, expect } from 'vitest';
import {
    normalize,
    pairedSeries,
    directionLabel,
    confidence,
    correlationSummary,
} from './correlation-view.js';

describe('normalize', () => {
    it('maps a series onto [0,1] preserving shape', () => {
        const n = normalize('ftr', [0, 5, 10])!;
        expect(n.points).toEqual([0, 0.5, 1]);
        expect([n.min, n.max]).toEqual([0, 10]);
    });

    it('keeps gaps as gaps rather than zero-filling them', () => {
        // A zero-fill would draw a crash to the floor on a day with no reading.
        const n = normalize('ftr', [2, null, 4])!;
        expect(n.points).toEqual([0, null, 1]);
    });

    it('puts a flat series mid-axis, not on the floor', () => {
        // range 0 would divide by zero; pinning to 0 would read as "collapsed"
        // when the metric simply never moved.
        expect(normalize('x', [7, 7, 7])!.points).toEqual([0.5, 0.5, 0.5]);
    });

    it('is null when there is nothing to draw', () => {
        expect(normalize('x', [])).toBeNull();
        expect(normalize('x', [null, null])).toBeNull();
        expect(normalize('x', [Number.NaN])).toBeNull();
    });

    it('normalises independently of magnitude, which is the point', () => {
        // A pair only shares a chart because scale was removed: ftr in [0,1] and
        // tokens in the millions produce the SAME normalised shape.
        const small = normalize('ftr', [0.1, 0.2, 0.3])!;
        const huge = normalize('tokens_per_day', [1_000_000, 2_000_000, 3_000_000])!;
        // Approximate, not exact: (0.2-0.1)/(0.3-0.1) is 0.5000000000000001 in
        // binary floating point. The shapes match; the last bit does not.
        small.points.forEach((v, i) => {
            expect(v).toBeCloseTo(huge.points[i] as number, 12);
        });
    });
});

describe('pairedSeries', () => {
    it('keeps only slots where BOTH metrics have a value', () => {
        // The rho was computed over paired observations, so the chart must show
        // the same days — otherwise the picture contradicts the number beside it.
        const p = pairedSeries('a', [1, 2, 3, 4], 'b', [10, null, 30, 40])!;
        expect(p.slots).toBe(3);
        expect(p.a.points[1]).toBeNull();
        expect(p.b.points[1]).toBeNull();
    });

    it('normalises each half over its own paired values only', () => {
        // `a`'s dropped slot held its max; excluding it must move the top of the
        // scale, or the visible line would never reach 1.
        const p = pairedSeries('a', [1, 99, 3], 'b', [5, null, 7])!;
        expect(p.a.max).toBe(3);
        expect(p.a.points).toEqual([0, null, 1]);
    });

    it('is null when fewer than two paired slots survive', () => {
        expect(pairedSeries('a', [1, null], 'b', [null, 2])).toBeNull();
        expect(pairedSeries('a', [1], 'b', [2])).toBeNull();
    });

    it('tolerates series of different lengths', () => {
        const p = pairedSeries('a', [1, 2, 3, 4, 5], 'b', [1, 2, 3])!;
        expect(p.a.points).toHaveLength(3);
    });
});

describe('reading aids', () => {
    it('names the direction', () => {
        expect(directionLabel(0.8)).toBe('together');
        expect(directionLabel(-0.8)).toBe('inversely');
        expect(directionLabel(0)).toBe('together');
    });

    it('grades confidence by sample size, not by rho', () => {
        // A strong rho over few days is still thin — magnitude is not evidence.
        expect(confidence(156)).toBe('strong');
        expect(confidence(40)).toBe('moderate');
        expect(confidence(26)).toBe('thin');
    });

    it('states the relationship and its limits in one line', () => {
        const s = correlationSummary({ a: 'ftr', b: 'rework_ratio', rho: -0.95, n: 156 }, (k) =>
            k === 'ftr' ? 'First-turn resolution' : 'Rework share',
        );
        expect(s).toContain('First-turn resolution falls as Rework share');
        expect(s).toContain('156 paired days');
        // The caveat is not optional — correlation is not causation, and the UI
        // must never imply otherwise.
        expect(s).toContain('not necessarily causal');
    });
});
