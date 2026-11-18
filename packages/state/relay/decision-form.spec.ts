// A form decision's form (#389 D6): the schema the lane sent passes through to
// `@rokkit/forms` unchanged; the layout is the lane's, else one built from the schema;
// the data starts from each `default`. The answer keeps what was filled and drops the
// optional fields left empty, so absent stays absent (D83).
import { describe, it, expect } from 'vitest';
import { patternMisses, toAnswerValues, toDecisionForm, toPatternRegExp, withoutPatterns } from './decision-form.js';
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
    expect(toDecisionForm({ ...response(), layout }).layout).toEqual(layout);
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

  it('drops optional properties left empty (undefined, null, "", a number input cleared to NaN)', () => {
    const r = response();
    expect(toAnswerValues(r.schema, { target: 'main', max_retries: Number.NaN, ship: null })).toEqual({ target: 'main' });
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

// The daemon checks a pattern with Rust `regex`, which reads text as Unicode; the inline check
// reads it the same way so it never refuses a value the daemon would take.
describe('toPatternRegExp', () => {
  it('reads text by code point, as the daemon does', () => {
    expect(toPatternRegExp('^.$')!.test('😀')).toBe(true);
    expect(toPatternRegExp('^[^/]{2}$')!.test('a😀')).toBe(true);
    expect(toPatternRegExp('^\\p{Lu}')!.test('Élan')).toBe(true);
    expect(toPatternRegExp('^\\p{Lu}')!.test('élan')).toBe(false);
  });

  it('lets . match a carriage return but not a newline, as Rust does', () => {
    expect(toPatternRegExp('^a.b$')!.test('a\rb')).toBe(true);
    expect(toPatternRegExp('^a.b$')!.test('a b')).toBe(true);
    expect(toPatternRegExp('^a.b$')!.test('a\nb')).toBe(false);
    // An escaped dot and a dot in a class stay literal.
    expect(toPatternRegExp('^a\\.b$')!.test('axb')).toBe(false);
    expect(toPatternRegExp('^[.]$')!.test('x')).toBe(false);
  });

  it('is unanchored, as JSON Schema says', () => {
    expect(toPatternRegExp('src/')!.test('a/src/b')).toBe(true);
  });

  it('is null when the browser cannot compile it, so the daemon decides', () => {
    expect(toPatternRegExp('\\#')).toBeNull();
  });
});

describe('patternMisses / withoutPatterns', () => {
  const schema = {
    type: 'object' as const,
    properties: {
      path: { type: 'string' as const, pattern: '^/' },
      name: { type: 'string' as const, pattern: '^\\p{L}+$' },
      odd: { type: 'string' as const, pattern: '\\#' },
      free: { type: 'string' as const },
    },
  };

  it('names each filled field whose text misses its pattern; empty fields and uncompiled patterns are left to others', () => {
    expect(patternMisses(schema, { path: 'rel', name: 'José', odd: 'x', free: 'y' })).toEqual(['path']);
    expect(patternMisses(schema, { path: '', name: 'a1' })).toEqual(['name']);
  });

  it('drops every pattern and nothing else', () => {
    const stripped = withoutPatterns(schema);
    expect(stripped.properties.path).toEqual({ type: 'string' });
    expect(stripped.properties.free).toEqual({ type: 'string' });
    expect(schema.properties.path.pattern).toBe('^/');
  });
});
