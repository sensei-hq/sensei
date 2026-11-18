// A form decision's form (#389 D6): what `@rokkit/forms` draws for a decision's `response`.
// The schema passes through unchanged (no sensei field format); the layout is the lane's,
// else one element per property in `properties` order; the data starts from each `default`.
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
  for (const [name, property] of Object.entries(response.schema.properties)) {
    if (property.default !== undefined) data[name] = property.default;
  }
  return { schema: response.schema, layout: response.layout ?? defaultLayout(response.schema), data };
}

/** Empty as a form leaves it: never set, cleared, or a number input emptied (NaN). */
function isEmpty(value: unknown): boolean {
  return value === undefined || value === null || value === '' || (typeof value === 'number' && Number.isNaN(value));
}

/** The answer's `values`: the schema's properties only; an empty optional one is left out. */
export function toAnswerValues(schema: DecisionSchema, data: Record<string, unknown>): Record<string, unknown> {
  const required = new Set(schema.required ?? []);
  const values: Record<string, unknown> = {};
  for (const name of Object.keys(schema.properties)) {
    const value = data[name];
    if (isEmpty(value) && !required.has(name)) continue;
    values[name] = value;
  }
  return values;
}
