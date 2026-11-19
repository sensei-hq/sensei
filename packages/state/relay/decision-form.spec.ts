// A form decision's form (#389 D6): the schema the lane sent passes through to
// `@rokkit/forms` unchanged; the layout is the lane's (or the daemon's, in the lane's order), else
// one built from the schema; an element without a label or help takes them from the schema;
// the data starts from each `default`. The answer keeps what was filled and drops the
// optional fields left empty, so absent stays absent (D83).
import { describe, it, expect } from 'vitest';
import { textMisses, toAnswerValues, toDecisionForm, toPatternMatcher, withoutTextRules } from './decision-form.js';
import type { RelayDecisionResponse } from './types.js';
import * as F from './fixtures.js';

const response = (): RelayDecisionResponse => F.formDecision().response!;

describe('toDecisionForm', () => {
  it('passes the schema through unchanged', () => {
    const r = response();
    expect(toDecisionForm(r).schema).toEqual(r.schema);
  });

  it("uses the lane's layout when it sent one", () => {
    const layout = {
      type: 'vertical' as const,
      elements: [
        { scope: '#/ship', label: 'Ship now', props: { placeholder: 'no' } },
        { scope: '#/target', label: 'Branch' },
        { scope: '#/max_retries' },
      ],
    };
    expect(toDecisionForm({ ...response(), layout }).layout).toEqual({
      type: 'vertical',
      elements: [
        { scope: '#/ship', label: 'Ship now', description: 'Ship it now?', props: { placeholder: 'no' } },
        { scope: '#/target', label: 'Branch' },
        { scope: '#/max_retries', label: 'Max retries' },
      ],
    });
  });

  it("keeps the layout's order, the daemon's record of the order the lane wrote (jsonb reorders properties)", () => {
    // What the daemon stores for a form sent without a layout: the scopes in the lane's order.
    const layout = {
      type: 'vertical' as const,
      elements: [{ scope: '#/target' }, { scope: '#/max_retries' }, { scope: '#/ship' }],
    };
    const r = response();
    r.schema.properties = { ship: r.schema.properties.ship, max_retries: r.schema.properties.max_retries, target: r.schema.properties.target };
    expect(toDecisionForm({ ...r, layout }).layout).toEqual({
      type: 'vertical',
      elements: [
        { scope: '#/target', label: 'Target branch' },
        { scope: '#/max_retries', label: 'Max retries' },
        { scope: '#/ship', label: 'Ship', description: 'Ship it now?' },
      ],
    });
  });

  it('else builds a vertical layout in properties order: label from title, else the name in words; description as help', () => {
    expect(toDecisionForm(response()).layout).toEqual({
      type: 'vertical',
      elements: [
        { scope: '#/target', label: 'Target branch' },
        { scope: '#/max_retries', label: 'Max retries' },
        { scope: '#/ship', label: 'Ship', description: 'Ship it now?' },
      ],
    });
  });

  it('seeds the data from each default, and nothing else', () => {
    expect(toDecisionForm(response()).data).toEqual({ max_retries: 2 });
  });

  it('a required yes/no with no default starts as no, as its unchecked box shows; an optional one stays unset', () => {
    const r = response();
    r.schema.required = ['target', 'ship'];
    expect(toDecisionForm(r).data).toEqual({ max_retries: 2, ship: false });
    expect(toDecisionForm(response()).data).toEqual({ max_retries: 2 });
  });

  it('a default of false still seeds the field', () => {
    const r = response();
    r.schema.properties.ship.default = false;
    expect(toDecisionForm(r).data).toEqual({ max_retries: 2, ship: false });
  });
});

describe('toAnswerValues', () => {
  it('keeps what was filled, false and 0 included', () => {
    const r = response();
    expect(toAnswerValues(r.schema, { target: 'main', max_retries: 0, ship: false })).toEqual({
      target: 'main',
      max_retries: 0,
      ship: false,
    });
  });

  it('drops optional properties left empty (undefined, null, "")', () => {
    const r = response();
    expect(toAnswerValues(r.schema, { target: 'main', max_retries: undefined, ship: null })).toEqual({ target: 'main' });
    expect(toAnswerValues(r.schema, { target: 'main', max_retries: '' })).toEqual({ target: 'main' });
  });

  it('keeps a required property even when empty, so the daemon names it', () => {
    const r = response();
    expect(toAnswerValues(r.schema, { target: '' })).toEqual({ target: '' });
  });

  it('sends only the schema properties', () => {
    const r = response();
    expect(toAnswerValues(r.schema, { target: 'main', stray: 1 })).toEqual({ target: 'main' });
  });
});

// The daemon checks a pattern with Rust `regex`, which reads text as Unicode in linear time; the
// inline check reads it the same way, so it never refuses a value the daemon would take, and never
// takes longer than the daemon would on a lane's pattern.
describe('toPatternMatcher', () => {
  const m = (pattern: string) => toPatternMatcher(pattern)!;

  it('reads text by code point, as the daemon does', () => {
    expect(m('^.$')('😀')).toBe(true);
    expect(m('^[^/]{2}$')('a😀')).toBe(true);
    expect(m('^\\p{Lu}')('Élan')).toBe(true);
    expect(m('^\\p{Lu}')('élan')).toBe(false);
    expect(m('^\\P{L}+$')('12-')).toBe(true);
    expect(m('^[\\p{Lu}0-9]+$')('É9')).toBe(true);
  });

  it('lets . match a carriage return but not a newline, as Rust does', () => {
    expect(m('^a.b$')('a\rb')).toBe(true);
    expect(m('^a.b$')('a b')).toBe(true);
    expect(m('^a.b$')('a\nb')).toBe(false);
    // An escaped dot and a dot in a class stay literal.
    expect(m('^a\\.b$')('axb')).toBe(false);
    expect(m('^[.]$')('x')).toBe(false);
  });

  it('is unanchored, as JSON Schema says', () => {
    expect(m('src/')('a/src/b')).toBe(true);
    expect(m('src/')('a/sr/b')).toBe(false);
  });

  it('reads the dialect the daemon allows: classes, ranges, alternation, groups and counts', () => {
    expect(m('^(feat|fix)/[a-z0-9-]+$')('fix/issue-389')).toBe(true);
    expect(m('^(feat|fix)/[a-z0-9-]+$')('chore/x')).toBe(false);
    expect(m('^[^a-c]+$')('xyz')).toBe(true);
    expect(m('^[^a-c]+$')('xbz')).toBe(false);
    expect(m('^a{2}$')('aa')).toBe(true);
    expect(m('^a{2}$')('aaa')).toBe(false);
    expect(m('^a{2,}$')('aaaa')).toBe(true);
    expect(m('^a{2,3}$')('aaaa')).toBe(false);
    expect(m('^(ab){1,2}c?$')('ababc')).toBe(true);
    expect(m('^x*?y+?$')('xxy')).toBe(true);
    expect(m('^\\(\\)\\[\\]\\{\\}\\*\\+\\?\\|\\^\\$\\\\\\/$')('()[]{}*+?|^$\\/')).toBe(true);
    expect(m('^a\\tb\\nc$')('a\tb\nc')).toBe(true);
    expect(m('^[a\\-z]$')('-')).toBe(true);
    expect(m('^[a\\-z]$')('b')).toBe(false);
    expect(m('^$')('')).toBe(true);
    expect(m('a|^$')('')).toBe(true);
    expect(m('()')('x')).toBe(true);
  });

  it('is null outside that dialect, so the daemon decides', () => {
    for (const pattern of ['\\#', '\\d', '(?i)a', '[[:alpha:]]', 'a{', '(a', 'a)', '[a', '\\p{Greek}']) {
      expect(toPatternMatcher(pattern), pattern).toBeNull();
    }
  });

  it('is null past its state cap, so no count can stall the form either', () => {
    expect(toPatternMatcher('a{20000}')).toBeNull();
    const wide = toPatternMatcher('(\\p{L}|\\p{N}|[a-z]){1,1600}!')!;
    const start = performance.now();
    expect(wide('abcd'.repeat(150))).toBe(false);
    expect(performance.now() - start).toBeLessThan(250);
  });

  // A lane's pattern may nest quantifiers or repeat overlapping ones: Rust `regex` runs it in
  // linear time and the daemon accepts it, but a backtracking RegExp takes exponential time and
  // would freeze the webview on Answer and on every edit while a message shows.
  it.each([
    ['^(a+)+$', 'a'.repeat(40) + '!'],
    ['^(a|a)+$', 'a'.repeat(40) + '!'],
    ['^(a|aa)*$', 'a'.repeat(60) + '!'],
    ['^([a-z]*)*$', 'a'.repeat(40) + '!'],
    ['(.*a){12}', 'a'.repeat(600)],
    ['a*a*a*a*a*a*a*a*b', 'a'.repeat(600)],
  ])('reads %s in linear time, as the daemon does', (pattern, text) => {
    const start = performance.now();
    expect(m(pattern)(text)).toBe(pattern === '(.*a){12}');
    expect(performance.now() - start).toBeLessThan(250);
  });
});

describe('textMisses / withoutTextRules', () => {
  const schema = {
    type: 'object' as const,
    properties: {
      path: { type: 'string' as const, pattern: '^/' },
      name: { type: 'string' as const, pattern: '^\\p{L}+$' },
      odd: { type: 'string' as const, pattern: '\\#' },
      short: { type: 'string' as const, maxLength: 2 },
      free: { type: 'string' as const },
    },
  };

  it('checks a catastrophic pattern without freezing', () => {
    const nested = { type: 'object' as const, properties: { p: { type: 'string' as const, pattern: '^(a+)+$' } } };
    const start = performance.now();
    expect(textMisses(nested, { p: 'a'.repeat(28) + '!' })).toEqual({ p: 'pattern' });
    expect(performance.now() - start).toBeLessThan(250);
  });

  it('names each filled field whose text misses its pattern; empty fields and uncompiled patterns are left to others', () => {
    expect(textMisses(schema, { path: 'rel', name: 'José', odd: 'x', free: 'y' })).toEqual({ path: 'pattern' });
    expect(textMisses(schema, { path: '', name: 'a1' })).toEqual({ name: 'pattern' });
  });

  // The daemon counts a maxLength in characters (code points), not UTF-16 units as `.length` does.
  it('counts maxLength in characters, as the daemon does', () => {
    expect(textMisses(schema, { short: '😀😀' })).toEqual({});
    expect(textMisses(schema, { short: 'é😀' })).toEqual({});
    expect(textMisses(schema, { short: '😀😀😀' })).toEqual({ short: 'maxLength' });
    expect(textMisses(schema, { short: 'abc' })).toEqual({ short: 'maxLength' });
  });

  it('reads maxLength before the pattern, as the daemon does', () => {
    const both = { type: 'object' as const, properties: { p: { type: 'string' as const, maxLength: 1, pattern: '^/' } } };
    expect(textMisses(both, { p: 'ab' })).toEqual({ p: 'maxLength' });
  });

  it('drops every pattern and maxLength and nothing else', () => {
    const stripped = withoutTextRules(schema);
    expect(stripped.properties.path).toEqual({ type: 'string' });
    expect(stripped.properties.short).toEqual({ type: 'string' });
    expect(stripped.properties.free).toEqual({ type: 'string' });
    expect(schema.properties.path.pattern).toBe('^/');
    expect(schema.properties.short.maxLength).toBe(2);
  });
});
