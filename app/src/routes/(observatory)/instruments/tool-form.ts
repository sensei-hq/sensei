// Observatory · Instruments — a tool's input form, read from the tool's own
// `inputSchema` as its MCP server declares it, and its answer as text. Pure.

export type FieldKind = 'string' | 'number' | 'boolean' | 'enum' | 'json';

export type ToolField = {
  name: string;
  kind: FieldKind;
  required: boolean;
  description: string | null;
  default: unknown;
  options: string[] | null;
};

type Prop = { type?: unknown; enum?: unknown; description?: unknown; default?: unknown };

function kindOf(p: Prop): FieldKind {
  if (Array.isArray(p.enum) && p.enum.every((o) => typeof o === 'string')) return 'enum';
  const type = Array.isArray(p.type) ? p.type.find((t) => t !== 'null') : p.type;
  if (type === 'integer' || type === 'number') return 'number';
  if (type === 'boolean') return 'boolean';
  if (type === 'object' || type === 'array') return 'json';
  return 'string';
}

/** One field per property of the schema, in its declared order. */
export function formFields(schema: unknown): ToolField[] {
  const s = schema as { properties?: Record<string, Prop>; required?: unknown } | undefined;
  if (!s?.properties || typeof s.properties !== 'object') return [];
  const required = new Set(Array.isArray(s.required) ? s.required : []);
  return Object.entries(s.properties).map(([name, p]) => ({
    name,
    kind: kindOf(p ?? {}),
    required: required.has(name),
    description: typeof p?.description === 'string' ? p.description : null,
    default: p?.default,
    options: Array.isArray(p?.enum) ? (p.enum as string[]) : null,
  }));
}

/** A repo or workspace id the schema leaves open: the one the user is in. */
const SCOPE_FIELD = /^(repo|workspace|project)_?id$/i;

/** Each field's starting value: its declared default, else the active
 *  workspace for a repo/workspace id, else empty. */
export function initialValues(fields: ToolField[], activeWorkspaceId: string | null): Record<string, string> {
  const values: Record<string, string> = {};
  for (const f of fields) {
    if (f.default !== undefined) {
      values[f.name] = f.kind === 'json' ? JSON.stringify(f.default) : String(f.default);
    } else if (activeWorkspaceId && f.kind === 'string' && SCOPE_FIELD.test(f.name)) {
      values[f.name] = activeWorkspaceId;
    } else {
      values[f.name] = '';
    }
  }
  return values;
}

export type Arguments = { ok: true; args: Record<string, unknown> } | { ok: false; error: string };

/** The form's values as the tool's arguments, each in its declared type. An
 *  empty optional value is left out — the tool's own default applies. */
export function toArguments(fields: ToolField[], values: Record<string, string>): Arguments {
  const args: Record<string, unknown> = {};
  for (const f of fields) {
    const raw = (values[f.name] ?? '').trim();
    if (raw === '') {
      if (f.required) return { ok: false, error: `${f.name} is required` };
      continue;
    }
    if (f.kind === 'number') {
      const n = Number(raw);
      if (!Number.isFinite(n)) return { ok: false, error: `${f.name} must be a number` };
      args[f.name] = n;
    } else if (f.kind === 'boolean') {
      args[f.name] = raw === 'true';
    } else if (f.kind === 'json') {
      try {
        args[f.name] = JSON.parse(raw);
      } catch {
        return { ok: false, error: `${f.name} must be JSON` };
      }
    } else {
      args[f.name] = raw;
    }
  }
  return { ok: true, args };
}

function pretty(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

/** A tool result as text: its text parts joined (each pretty-printed when it
 *  is JSON); a result with any other part is shown whole. */
export function resultText(result: unknown): string {
  const content = (result as { content?: unknown })?.content;
  if (Array.isArray(content) && content.length > 0 && content.every((c) => c?.type === 'text' && typeof c.text === 'string')) {
    return content.map((c) => pretty(c.text)).join('\n');
  }
  return JSON.stringify(result, null, 2);
}
