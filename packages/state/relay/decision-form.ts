// A form decision's form (#389 D6): what `@rokkit/forms` draws for a decision's `response`.
// The schema passes through unchanged (no sensei field format). The layout is the one on the
// wire: the lane's, or, for a lane that sent none, the daemon's list of the properties in the
// order the lane wrote them (jsonb stores `properties` shorter-first, so their key order is not
// the lane's). Each element without a label or help takes them from its property's `title`
// (else its name in words) and `description`. The data starts from each `default`, and a
// required yes/no with none starts as no, as its unchecked box shows.
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

/** The wire's layout, else one element per property in key order; labels and help from the schema. */
function formLayout(response: RelayDecisionResponse): RelayDecisionLayout {
  const { properties } = response.schema;
  const elements = response.layout?.elements ?? Object.keys(properties).map((name) => ({ scope: `#/${name}` }));
  return {
    type: 'vertical',
    elements: elements.map((element) => {
      const property = properties[element.scope.slice(2)];
      const description = element.description ?? property?.description;
      return {
        ...element,
        label: element.label ?? property?.title ?? inWords(element.scope.slice(2)),
        ...(description ? { description } : {}),
      };
    }),
  };
}

export function toDecisionForm(response: RelayDecisionResponse): DecisionFormView {
  const data: Record<string, unknown> = {};
  const required = new Set(response.schema.required ?? []);
  for (const [name, property] of Object.entries(response.schema.properties)) {
    if (property.default !== undefined) data[name] = property.default;
    else if (property.type === 'boolean' && required.has(name)) data[name] = false;
  }
  return { schema: response.schema, layout: formLayout(response), data };
}

/** Empty as a form leaves it: never set, or cleared. */
function isEmptyValue(value: unknown): boolean {
  return value === undefined || value === null || value === '';
}

/** The Unicode general categories `\p{..}` may name (`PATTERN_CATEGORIES` in the daemon). */
const PATTERN_CATEGORIES = [
  'L', 'Lu', 'Ll', 'Lt', 'Lm', 'Lo', 'M', 'Mn', 'Mc', 'Me', 'N', 'Nd', 'Nl', 'No', 'P', 'Pc',
  'Pd', 'Ps', 'Pe', 'Pi', 'Pf', 'Po', 'S', 'Sm', 'Sc', 'Sk', 'So', 'Z', 'Zs', 'Zl', 'Zp',
] as const;

/** One code point's test against a general category: a fixed, single-character expression. */
const categoryTests = new Map<string, RegExp>(
  PATTERN_CATEGORIES.map((name) => [name, new RegExp(`^\\p{${name}}$`, 'u')]), // nosemgrep: javascript.lang.security.audit.detect-non-literal-regexp.detect-non-literal-regexp
);

/** The escaped syntax characters that stand for themselves, and the control escapes. */
const LITERAL_ESCAPES = new Set('^$\\.*+?()[]{}|/');
const CONTROL_ESCAPES: Record<string, number> = { n: 0x0a, t: 0x09, r: 0x0d, f: 0x0c, v: 0x0b };

/** The most NFA states a pattern may expand to (counts copy their body); past it, the daemon decides. */
const MAX_PATTERN_STATES = 10_000;

type CodeTest = (cp: number) => boolean;
type PatternNode =
  | { k: 'char'; test: CodeTest }
  | { k: 'bol' | 'eol' }
  | { k: 'cat' | 'alt'; items: PatternNode[] }
  | { k: 'rep'; node: PatternNode; min: number; max: number };

/** A pattern outside the dialect both engines read alike. */
class OutsideDialect extends Error {}

/**
 * Parse a lane's pattern in the dialect the daemon allows (`check_pattern_dialect`): literals,
 * escaped syntax characters, `\n \t \r \f \v`, `\p{X}`/`\P{X}` of a general category, plain
 * classes with ranges, `.` (anything but a newline, as in Rust), `^`/`$` (start and end of
 * text), groups, `|`, and `* + ? {m} {m,} {m,n}` (each optionally lazy).
 */
function parsePattern(pattern: string): PatternNode {
  const cs = [...pattern];
  let i = 0;
  const fail = (): never => {
    throw new OutsideDialect();
  };
  const take = (): number => cs[i++].codePointAt(0)!;

  function category(): CodeTest {
    // At `p`/`P` after the backslash.
    const negated = cs[i] === 'P';
    i++;
    if (cs[i] !== '{') fail();
    const close = cs.indexOf('}', i);
    if (close < 0) fail();
    const re = categoryTests.get(cs.slice(i + 1, close).join('')) ?? fail();
    i = close + 1;
    return (cp) => re.test(String.fromCodePoint(cp)) !== negated;
  }

  /** An escape's code point, or its category test; `i` is at the backslash. */
  function escape(inClass: boolean): number | CodeTest {
    i++;
    const c = cs[i] ?? fail();
    if (c === 'p' || c === 'P') return category();
    i++;
    if (LITERAL_ESCAPES.has(c) || (inClass && c === '-')) return c.codePointAt(0)!;
    return CONTROL_ESCAPES[c] ?? fail();
  }

  function classMember(): number | CodeTest {
    return cs[i] === '\\' ? escape(true) : take();
  }

  function charClass(): CodeTest {
    i++; // `[`
    const negated = cs[i] === '^';
    if (negated) i++;
    if (cs[i] === ']') fail();
    const tests: CodeTest[] = [];
    for (;;) {
      const c = cs[i] ?? fail();
      if (c === ']') break;
      if (c === '[') fail();
      if ((c === '&' || c === '-' || c === '~') && cs[i + 1] === c) fail();
      const lo = classMember();
      if (typeof lo !== 'number') {
        tests.push(lo);
      } else if (cs[i] === '-' && cs[i + 1] !== undefined && cs[i + 1] !== ']') {
        i++;
        const hi = classMember();
        if (typeof hi !== 'number' || hi < lo) fail();
        tests.push((cp) => cp >= lo && cp <= (hi as number));
      } else {
        tests.push((cp) => cp === lo);
      }
    }
    i++; // `]`
    return (cp) => tests.some((t) => t(cp)) !== negated;
  }

  function count(): { min: number; max: number } | null {
    const c = cs[i];
    if (c === '*' || c === '+' || c === '?') {
      i++;
      return { min: c === '+' ? 1 : 0, max: c === '?' ? 1 : Infinity };
    }
    if (c !== '{') return null;
    const close = cs.indexOf('}', i);
    const m = close < 0 ? null : cs.slice(i + 1, close).join('').match(/^(\d+)(,(\d*))?$/);
    if (!m) return fail();
    const min = Number(m[1]);
    const max = m[2] === undefined ? min : m[3] === '' ? Infinity : Number(m[3]);
    if (max < min) fail();
    i = close + 1;
    return { min, max };
  }

  function atom(): PatternNode {
    const c = cs[i];
    if (c === '(') {
      i++;
      const node = alternation();
      if (cs[i] !== ')') fail();
      i++;
      return node;
    }
    if (c === '[') return { k: 'char', test: charClass() };
    if (c === '\\') {
      const e = escape(false);
      return { k: 'char', test: typeof e === 'number' ? (cp) => cp === e : e };
    }
    if ('])}{*+?'.includes(c)) fail();
    const cp = take();
    if (c === '.') return { k: 'char', test: (x) => x !== 0x0a };
    if (c === '^') return { k: 'bol' };
    if (c === '$') return { k: 'eol' };
    return { k: 'char', test: (x) => x === cp };
  }

  function sequence(): PatternNode {
    const items: PatternNode[] = [];
    while (i < cs.length && cs[i] !== '|' && cs[i] !== ')') {
      const node = atom();
      const n = count();
      if (n === null) {
        items.push(node);
        continue;
      }
      if (node.k === 'bol' || node.k === 'eol') fail();
      if (cs[i] === '?') i++; // lazy: the same yes/no as greedy
      if (count() !== null) fail(); // a count of a count does not compile in the browser
      items.push({ k: 'rep', node, ...n });
    }
    return items.length === 1 ? items[0] : { k: 'cat', items };
  }

  function alternation(): PatternNode {
    const items = [sequence()];
    while (cs[i] === '|') {
      i++;
      items.push(sequence());
    }
    return items.length === 1 ? items[0] : { k: 'alt', items };
  }

  const root = alternation();
  if (i < cs.length) fail();
  return root;
}

type NfaState =
  | { t: 'char'; test: CodeTest; out: number }
  | { t: 'split'; out: number; out2: number }
  | { t: 'bol' | 'eol'; out: number }
  | { t: 'match' };

/** Thompson's construction: each node becomes states whose dangling exits are patched to what follows. */
function toNfa(root: PatternNode): { states: NfaState[]; start: number } {
  const states: NfaState[] = [];
  type Hole = { state: number; key: 'out' | 'out2' };
  // A fragment: its entry state (null when it matches empty without a state) and its exits.
  type Frag = { start: number | null; holes: Hole[] };
  const EMPTY: Frag = { start: null, holes: [] };
  const add = (s: NfaState): number => {
    if (states.length >= MAX_PATTERN_STATES) throw new OutsideDialect();
    states.push(s);
    return states.length - 1;
  };
  const patch = (holes: Hole[], to: number) => {
    for (const h of holes) (states[h.state] as Record<Hole['key'], number>)[h.key] = to;
  };
  const join = (a: Frag, b: Frag): Frag => {
    if (a.start === null) return b;
    if (b.start === null) return a;
    patch(a.holes, b.start);
    return { start: a.start, holes: b.holes };
  };
  /** Either `a` or `b`: a split whose empty side becomes an exit. */
  const either = (a: Frag, b: Frag): Frag => {
    const s = add({ t: 'split', out: a.start ?? -1, out2: b.start ?? -1 });
    const holes: Hole[] = [...a.holes, ...b.holes];
    if (a.start === null) holes.push({ state: s, key: 'out' });
    if (b.start === null) holes.push({ state: s, key: 'out2' });
    return { start: s, holes };
  };
  /** `f*`: a split that enters `f` and loops back after it. */
  const star = (f: Frag): Frag => {
    if (f.start === null) return EMPTY;
    const s = add({ t: 'split', out: f.start, out2: -1 });
    patch(f.holes, s);
    return { start: s, holes: [{ state: s, key: 'out2' }] };
  };

  function build(node: PatternNode): Frag {
    switch (node.k) {
      case 'char': {
        const s = add({ t: 'char', test: node.test, out: -1 });
        return { start: s, holes: [{ state: s, key: 'out' }] };
      }
      case 'bol':
      case 'eol': {
        const s = add({ t: node.k, out: -1 });
        return { start: s, holes: [{ state: s, key: 'out' }] };
      }
      case 'cat':
        return node.items.reduce((acc, item) => join(acc, build(item)), EMPTY);
      case 'alt':
        return node.items.map(build).reduceRight((rest, item) => either(item, rest));
      case 'rep': {
        // A body that adds no state matches only empty, and so does any count of it: return
        // before the loops, which the state cap cannot bound when no copy adds a state.
        if (node.max === 0) return EMPTY;
        let spare: Frag | null = build(node.node);
        if (spare.start === null) return EMPTY;
        // Each copy after the first is built afresh; the first is the one built above.
        const copy = (): Frag => {
          const f = spare ?? build(node.node);
          spare = null;
          return f;
        };
        let frag = EMPTY;
        for (let k = 0; k < node.min; k++) frag = join(frag, copy());
        if (node.max === Infinity) return join(frag, star(copy()));
        // x{m,n}: n - m optional copies, each nested in the one before.
        let tail = EMPTY;
        for (let k = node.min; k < node.max; k++) tail = either(join(copy(), tail), EMPTY);
        return join(frag, tail);
      }
    }
  }

  const frag = build(root);
  const end = add({ t: 'match' });
  if (frag.start === null) return { states, start: end };
  patch(frag.holes, end);
  return { states, start: frag.start };
}

/**
 * A lane's `pattern` read as the daemon reads it (Rust `regex`): unanchored, by code point, each
 * bare `.` matching anything but a newline, and in time linear in the text, so no pattern can
 * stall the form (a backtracking `RegExp` takes exponential time on `^(a+)+$`). The text runs
 * through a Thompson NFA, one state set per character. Null for a pattern outside the dialect
 * the daemon allows (`check_pattern_dialect`) or past `MAX_PATTERN_STATES`: the inline check then
 * leaves the pattern to the daemon.
 */
export function toPatternMatcher(pattern: string): ((text: string) => boolean) | null {
  let nfa: { states: NfaState[]; start: number };
  try {
    nfa = toNfa(parsePattern(pattern));
  } catch (e) {
    if (e instanceof OutsideDialect) return null;
    throw e;
  }
  const { states, start } = nfa;
  return (text) => {
    const cps = [...text].map((c) => c.codePointAt(0)!);
    const seen = new Int32Array(states.length).fill(-1);
    let matched = false;
    // Follow every empty step from `s` at `pos`, collecting the states that read a character.
    const enter = (list: number[], s: number, pos: number) => {
      const stack = [s];
      while (stack.length > 0) {
        const at = stack.pop()!;
        if (seen[at] === pos) continue;
        seen[at] = pos;
        const st = states[at];
        if (st.t === 'match') matched = true;
        else if (st.t === 'char') list.push(at);
        else if (st.t === 'split') stack.push(st.out2, st.out);
        else if (st.t === 'bol' ? pos === 0 : pos === cps.length) stack.push(st.out);
      }
    };
    let current: number[] = [];
    for (let pos = 0; ; pos++) {
      enter(current, start, pos); // unanchored: a match may begin anywhere
      if (matched) return true;
      if (pos === cps.length) return false;
      const next: number[] = [];
      for (const s of current) {
        const st = states[s] as { test: CodeTest; out: number };
        if (st.test(cps[pos])) enter(next, st.out, pos + 1);
      }
      current = next;
    }
  };
}

/**
 * The schema without the text rules the browser reads otherwise than the daemon: rokkit compiles
 * a `pattern` without `u` and counts `maxLength` in UTF-16 units (as a native `maxlength` does),
 * where the daemon counts characters. The form checks both itself (`textMisses`).
 */
export function withoutTextRules(schema: DecisionSchema): DecisionSchema {
  const properties = Object.fromEntries(
    Object.entries(schema.properties).map(([name, property]) => {
      const { pattern: _pattern, maxLength: _maxLength, ...rest } = property;
      return [name, rest];
    }),
  ) as DecisionSchema['properties'];
  return { ...schema, properties };
}

/** The text rule a filled field breaks, read as the daemon reads it: maxLength first, then pattern. */
export type TextMiss = 'maxLength' | 'pattern';

/** Each filled text field that breaks its maxLength (in characters) or its pattern. */
export function textMisses(schema: DecisionSchema, data: Record<string, unknown>): Record<string, TextMiss> {
  const misses: Record<string, TextMiss> = {};
  for (const [name, property] of Object.entries(schema.properties)) {
    const value = data[name];
    if (typeof value !== 'string' || value === '') continue;
    if (property.maxLength !== undefined && [...value].length > property.maxLength) {
      misses[name] = 'maxLength';
      continue;
    }
    const matches = property.pattern ? toPatternMatcher(property.pattern) : null;
    if (matches !== null && !matches(value)) misses[name] = 'pattern';
  }
  return misses;
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
