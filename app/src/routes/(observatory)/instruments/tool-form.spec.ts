import { describe, expect, it } from 'vitest';
import { formFields, initialValues, toArguments, resultText } from './tool-form';

const schema = {
  type: 'object',
  properties: {
    query: { type: 'string', description: 'What to look for' },
    limit: { type: 'integer', default: 10 },
    exact: { type: 'boolean' },
    scope: { type: 'string', enum: ['repo', 'workspace'] },
    filters: { type: 'object' },
    repoId: { type: 'string' },
  },
  required: ['query'],
};

describe('formFields', () => {
  it('reads one field per property, typed from the schema', () => {
    const fields = formFields(schema);
    expect(fields.map((f) => [f.name, f.kind, f.required])).toEqual([
      ['query', 'string', true],
      ['limit', 'number', false],
      ['exact', 'boolean', false],
      ['scope', 'enum', false],
      ['filters', 'json', false],
      ['repoId', 'string', false],
    ]);
    expect(fields[0].description).toBe('What to look for');
    expect(fields[3].options).toEqual(['repo', 'workspace']);
  });

  it('a tool with no schema, or no properties, has no fields', () => {
    expect(formFields(undefined)).toEqual([]);
    expect(formFields({ type: 'object' })).toEqual([]);
  });
});

describe('initialValues', () => {
  it('takes the declared default, and the active workspace for a repo id', () => {
    const v = initialValues(formFields(schema), 'ws-1');
    expect(v.limit).toBe('10');
    expect(v.repoId).toBe('ws-1');
    expect(v.query).toBe('');
    expect(v.exact).toBe('');
  });
});

describe('toArguments', () => {
  const fields = formFields(schema);

  it('sends each value as its declared type, and leaves out what is empty', () => {
    const got = toArguments(fields, {
      query: 'scan', limit: '5', exact: 'true', scope: 'repo', filters: '{"a":1}', repoId: '',
    });
    expect(got).toEqual({
      ok: true,
      args: { query: 'scan', limit: 5, exact: true, scope: 'repo', filters: { a: 1 } },
    });
  });

  it('refuses a missing required value, a number that is not one, and bad JSON', () => {
    expect(toArguments(fields, { query: '' })).toEqual({ ok: false, error: 'query is required' });
    expect(toArguments(fields, { query: 'q', limit: 'ten' })).toEqual({ ok: false, error: 'limit must be a number' });
    expect(toArguments(fields, { query: 'q', filters: '{' })).toEqual({ ok: false, error: 'filters must be JSON' });
  });
});

describe('resultText', () => {
  it('joins a result’s text parts, and pretty-prints anything else', () => {
    expect(resultText({ content: [{ type: 'text', text: 'a' }, { type: 'text', text: 'b' }] })).toBe('a\nb');
    expect(resultText({ content: [{ type: 'image', data: 'x' }] })).toBe(
      JSON.stringify({ content: [{ type: 'image', data: 'x' }] }, null, 2),
    );
  });

  it('pretty-prints a text part that is JSON', () => {
    expect(resultText({ content: [{ type: 'text', text: '{"a":1}' }] })).toBe('{\n  "a": 1\n}');
  });
});
