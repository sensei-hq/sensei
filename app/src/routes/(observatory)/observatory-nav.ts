/**
 * Observatory rail data — pure, no runes.
 *
 * Shapes the entries for the rokkit `List` (see ObservatorySidebar.svelte) and
 * resolves which entry the current route should highlight. Kept separate from
 * the component so the structure + active-matching are unit-testable without
 * mounting the List or the SvelteKit `page` store.
 *
 * Rail layout mirrors docs/mockups/Sensei/lib/observatory.jsx (~L247-323).
 */

/** A navigable rail entry. `value` mirrors `href` so List's value-match
 * (`proxy.value === value`) highlights the active route. */
export interface NavLink {
  kanji: string;
  text: string;
  href: string;
  value: string;
  badge?: string | number;
  alert?: boolean;
}

/** A static cluster label with its children (List `collapsible={false}`). */
export interface NavGroup {
  text: string;
  children: NavLink[];
}

/** Thin divider rendered by List as `<hr data-list-separator>`. */
export interface NavSeparator {
  type: 'separator';
}

export type NavEntry = NavLink | NavGroup | NavSeparator;

export interface NavOptions {
  /** Focus mode collapses the rail to anchors + "Needs you". */
  focus: boolean;
  /** Live project count for the Projects badge (mock until wired). */
  projectCount?: number;
}

const link = (
  kanji: string,
  text: string,
  href: string,
  extra: Partial<Pick<NavLink, 'badge' | 'alert'>> = {},
): NavLink => ({ kanji, text, href, value: href, ...extra });

// MOCK: badge counts are placeholders until wired to the daemon API.
const MOCK = {
  insights: '6',
  memories: '7',
  impact: '3',
  traceability: '4',
  upgrades: '5',
  sessions: '41',
  libraries: '14',
} as const;

/**
 * Build the rail entries. In Focus mode the Review group, the separator and
 * Preferences are dropped — leaving anchors + "Needs you" (just what needs a
 * decision). Dōjō is omitted (standalone-deferred).
 */
export function buildNavItems({ focus, projectCount }: NavOptions): NavEntry[] {
  const entries: NavEntry[] = [
    // Anchors — where every day starts.
    link('家', 'Today', '/'),
    link('場', 'Projects', '/projects', { badge: projectCount }),
    // Needs you — the daily payoff: everything with a pending decision.
    {
      text: 'Needs you',
      children: [
        link('今', 'Insights', '/insights', { badge: MOCK.insights }),
        link('覚', 'Memories', '/learnings', { badge: MOCK.memories }),
        link('果', 'Impact', '/impact', { badge: MOCK.impact, alert: true }),
        link('巻', 'Traceability', '/traceability', { badge: MOCK.traceability }),
        link('贈', 'Upgrades', '/upgrades', { badge: MOCK.upgrades }),
      ],
    },
  ];

  if (!focus) {
    // Review & diagnostics — reached periodically, hidden in Focus.
    entries.push({
      text: 'Review',
      children: [
        link('録', 'Sessions', '/sessions', { badge: MOCK.sessions }),
        link('庫', 'Libraries', '/libraries', { badge: MOCK.libraries }),
        link('具', 'Instruments', '/instruments'),
        link('診', 'Logs', '/logs'),
      ],
    });
    // Settings — visited when something needs changing, hidden in Focus.
    entries.push({ type: 'separator' });
    entries.push(link('調', 'Preferences', '/settings'));
  }

  return entries;
}

/** Flatten every link href from the full rail (focus-independent). */
function allHrefs(): string[] {
  const out: string[] = [];
  for (const e of buildNavItems({ focus: false })) {
    if ('href' in e) out.push(e.href);
    else if ('children' in e) for (const c of e.children) out.push(c.href);
  }
  return out;
}

/**
 * Which rail href should be highlighted for `pathname`. Exact match wins;
 * otherwise the longest non-root href that is a path prefix (so `/projects/123`
 * highlights Projects). `/` (Today) only matches exactly. Empty string when
 * nothing matches.
 */
export function resolveActiveHref(pathname: string): string {
  const hrefs = allHrefs();
  if (hrefs.includes(pathname)) return pathname;

  let best = '';
  for (const href of hrefs) {
    if (href === '/') continue;
    if (pathname.startsWith(href + '/') && href.length > best.length) best = href;
  }
  return best;
}
