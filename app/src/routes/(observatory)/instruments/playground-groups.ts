/**
 * The Playground rail's model: sensei's own tools, then one group per MCP
 * server discovered in an assistant's config (#84 T2), and the discovered
 * servers grouped by assistant for the section that manages them.
 *
 * Pure — the page passes the store's values in.
 */

export type PlaygroundTool = {
  id: string;
  mcp: string;
  name: string;
  kind: 'query' | 'action' | 'other';
  summary: string;
};

export type PlaygroundGroup = {
  id: string;
  kanji: string;
  /** Icon-mode mark (#133); `null` for an assistant's own mark, an identity
   *  that stays kanji in both modes. */
  icon?: string | null;
  name: string;
  installed: boolean;
  source: 'sensei' | 'discovered';
  serverId?: string;
  tools: PlaygroundTool[];
  /** Why a discovered server's tools are not shown: the read failed, or the
   *  daemon's probe did (#288). null when there is nothing to say. */
  error?: string | null;
};

/** The fields of a discovered server this model reads. */
export type DiscoveredServer = { id: string; acp_family: string; mcp_key: string; enabled: boolean };

/** The tool shape of sensei's own catalog this model reads. */
export type SenseiTool = { name: string; kind?: string; summary: string };

/** A cached manifest, as probed from the server: its tools are untyped JSON,
 *  and `error` is the probe's failure when it had one. */
export type ServerManifest = { tools?: unknown[]; error?: string | null };

/** A manifest entry this rail can show: one with a string name. */
function namedTool(t: unknown): t is { name: string; description?: unknown } {
  return typeof t === 'object' && t !== null && typeof (t as { name?: unknown }).name === 'string'
    && (t as { name: string }).name !== '';
}

/** Assistants in the order the section lists them, with their glyph and label. */
const FAMILIES: Array<{ family: string; label: string; kanji: string }> = [
  { family: 'claude', label: 'Claude', kanji: '書' },
  { family: 'cursor', label: 'Cursor', kanji: '刀' },
  { family: 'zed', label: 'Zed', kanji: '禅' },
];
const UNKNOWN_KANJI = '器';
/** Icon-mode marks (#133). The assistants keep their kanji: they are
 *  identities, and Solar has no logo for them. */
const SENSEI_ICON = 'i-solar-ruler-pen-bold-duotone';
const UNKNOWN_ICON = 'i-solar-plug-circle-bold-duotone';

/** Third-party catalogs rarely declare a kind: a mutating verb is an action,
 *  a getter / list / search a query. */
export function inferKind(name: string, declared: string | undefined): PlaygroundTool['kind'] {
  if (declared === 'action' || declared === 'query') return declared;
  const n = name.toLowerCase();
  if (/^(get|list|search|find|read|show|describe|inspect|explain|status|check)/.test(n)) return 'query';
  if (/^(set|create|delete|update|write|run|call|start|stop|restart|install|remove)/.test(n)) return 'action';
  return 'other';
}

export function buildPlaygroundGroups(
  senseiTools: SenseiTool[],
  servers: DiscoveredServer[],
  manifests: Record<string, ServerManifest | undefined>,
  readErrors: Record<string, string | undefined> = {},
): PlaygroundGroup[] {
  const groups: PlaygroundGroup[] = [
    {
      id: 'sensei',
      kanji: '具',
      icon: SENSEI_ICON,
      name: 'Sensei',
      installed: true,
      source: 'sensei',
      tools: senseiTools.map((t) => ({
        id: `sensei:${t.name}`,
        mcp: 'sensei',
        name: t.name,
        kind: t.kind === 'action' || t.kind === 'query' ? t.kind : 'other',
        summary: t.summary,
      })),
    },
  ];
  for (const server of servers) {
    const tools = (manifests[server.id]?.tools ?? [])
      .filter(namedTool)
      .map((t) => ({
        id: `${server.id}:${t.name}`,
        mcp: server.id,
        name: t.name,
        kind: inferKind(t.name, undefined),
        summary: typeof t.description === 'string' ? t.description : '',
      }));
    const family = FAMILIES.find((f) => f.family === server.acp_family);
    groups.push({
      id: server.id,
      kanji: family?.kanji ?? UNKNOWN_KANJI,
      icon: family ? null : UNKNOWN_ICON,
      name: server.mcp_key,
      installed: server.enabled,
      source: 'discovered',
      serverId: server.id,
      tools,
      error: readErrors[server.id] ?? manifests[server.id]?.error ?? null,
    });
  }
  return groups;
}

/** Discovered servers by assistant: known assistants first in a fixed order,
 *  any other family after them under its own name — a server is never dropped. */
export function serversByFamily<S extends DiscoveredServer>(
  servers: S[],
): Array<{ family: string; label: string; servers: S[] }> {
  const byFamily = new Map<string, S[]>();
  for (const s of servers) byFamily.set(s.acp_family, [...(byFamily.get(s.acp_family) ?? []), s]);
  const rank = (family: string) => {
    const i = FAMILIES.findIndex((f) => f.family === family);
    return i === -1 ? FAMILIES.length : i;
  };
  return [...byFamily.entries()]
    .sort(([a], [b]) => rank(a) - rank(b) || a.localeCompare(b))
    .map(([family, rows]) => ({
      family,
      label: FAMILIES.find((f) => f.family === family)?.label ?? family,
      servers: [...rows].sort((a, b) => a.mcp_key.localeCompare(b.mcp_key)),
    }));
}
