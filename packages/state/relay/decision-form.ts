// A form decision's form (#389 D6): what `@rokkit/forms` draws for a decision's `response`.
// The schema passes through unchanged (no sensei field format); the layout is the lane's,
// else one element per property in the order the wire lists `properties` (jsonb's order, not
// the lane's: a lane that needs an order sends a layout); the data starts from each `default`,
// and a required yes/no with none starts as no, as its unchecked box shows.
// `toAnswerValues` keeps what was filled and drops optional fields left empty, so absent stays
// absent (D83). The daemon re-validates every answer (D3); this is the person's convenience.
import type { RelayDecisionLayout, RelayDecisionResponse } from './types.js';

type DecisionSchema = RelayDecisionResponse['schema'];

/** What the form draws: the lane's schema, a layout, and the seeded data. */
export interface DecisionFormView {
  schema: DecisionSchema;
  layout: RelayDecisionLayout;
  data: Record<string, unknown>;
}

/** `max_retries` → `Max retries`. */
function inWords(name: string): string {
  const words = name.replace(/_/g, ' ').trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function defaultLayout(schema: DecisionSchema): RelayDecisionLayout {
  return {
    type: 'vertical',
    elements: Object.entries(schema.properties).map(([name, property]) => ({
      scope: `#/${name}`,
      label: property.title ?? inWords(name),
      ...(property.description ? { description: property.description } : {}),
    })),
  };
}

export function toDecisionForm(response: RelayDecisionResponse): DecisionFormView {
  const data: Record<string, unknown> = {};
  const required = new Set(response.schema.required ?? []);
  for (const [name, property] of Object.entries(response.schema.properties)) {
    if (property.default !== undefined) data[name] = property.default;
    else if (property.type === 'boolean' && required.has(name)) data[name] = false;
  }
  return { schema: response.schema, layout: response.layout ?? defaultLayout(response.schema), data };
}

/** Empty as a form leaves it: never set, or cleared. */
function isEmptyValue(value: unknown): boolean {
  return value === undefined || value === null || value === '';
}

/**
 * A lane's `pattern` read as the daemon reads it (Rust `regex`): with the `u` flag, so text is
 * read by code point and `\p{..}` works, and with each bare `.` matching anything but a newline
 * (the browser's `.` also skips `\r`, U+2028 and U+2029). The daemon refuses at ask time the
 * constructs the two engines read differently (`check_pattern_dialect`). Null when the browser
 * cannot compile it: the inline check then leaves the pattern to the daemon.
 */
export function toPatternRegExp(pattern: string): RegExp | null {
  let source = '';
  let inClass = false;
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === '\\') {
      source += c + (pattern[i + 1] ?? '');
      i++;
    } else if (inClass) {
      if (c === ']') inClass = false;
      source += c;
    } else if (c === '[') {
      inClass = true;
      source += c;
    } else {
      source += c === '.' ? '[^\\n]' : c;
    }
  }
  try {
    // The lane's pattern is the point of the check (≤ 200 chars, tested on ≤ 600 chars of text),
    // as it was in rokkit's own `new RegExp(schema.pattern)`.
    return new RegExp(source, 'u'); // nosemgrep: javascript.lang.security.audit.detect-non-literal-regexp.detect-non-literal-regexp
  } catch {
    return null;
  }
}

/** The schema without its `pattern`s: rokkit compiles them without `u`, so the form checks them itself. */
export function withoutPatterns(schema: DecisionSchema): DecisionSchema {
  const properties = Object.fromEntries(
    Object.entries(schema.properties).map(([name, property]) => {
      const { pattern: _pattern, ...rest } = property;
      return [name, rest];
    }),
  ) as DecisionSchema['properties'];
  return { ...schema, properties };
}

/** Each filled text field whose value misses its pattern, read as the daemon reads it. */
export function patternMisses(schema: DecisionSchema, data: Record<string, unknown>): string[] {
  return Object.entries(schema.properties)
    .filter(([name, property]) => {
      const value = data[name];
      if (!property.pattern || typeof value !== 'string' || value === '') return false;
      const re = toPatternRegExp(property.pattern);
      return re !== null && !re.test(value);
    })
    .map(([name]) => name);
}

/** The answer's `values`: the schema's properties only; an empty optional one is left out. */
export function toAnswerValues(schema: DecisionSchema, data: Record<string, unknown>): Record<string, unknown> {
  const required = new Set(schema.required ?? []);
  const values: Record<string, unknown> = {};
  for (const name of Object.keys(schema.properties)) {
    const value = data[name];
    if (isEmptyValue(value) && !required.has(name)) continue;
    values[name] = value;
  }
  return values;
}
