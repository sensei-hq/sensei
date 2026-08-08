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
  snippet?: string;
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
  type: "separator";
}

export type NavEntry = NavLink | NavGroup | NavSeparator;

export interface NavOptions {
  /** Focus mode collapses the rail to anchors + "Needs you". */
  focus: boolean;
  /** Live project count for the Projects badge (from `appState.projectCount`). */
  projectCount?: number;
}

const link = (
  kanji: string,
  text: string,
  href: string,
  extra: Partial<Pick<NavLink, "badge" | "alert">> = {},
): NavLink => ({ kanji, text, href, value: href, ...extra });

// Badge counts and the Impact alert were hardcoded MOCK placeholders — a
// fabricated "6 insights / 3 impact / alert" that lied about pending work when
// nothing was pending (the #109 fabrication audit). Removed until wired to real
// daemon counts. The only live badge is Projects (`projectCount` from app state);
// each other rail entry shows its count on its own screen, honestly.

/**
 * Build the rail entries. In Focus mode the Review group, the separator and
 * Preferences are dropped — leaving anchors + "Needs you" (just what needs a
 * decision). Dōjō connections sit in Review (reached periodically, hidden in
 * Focus) — the entry point for pairing with an employer/client/community Dōjō.
 */
export function buildNavItems({ focus, projectCount }: NavOptions): NavEntry[] {
  const entries: NavEntry[] = [
    // Anchors — where every day starts (top-level; the "Observatory" header +
    // All|Focus toggle live in the sidebar chrome, not the List — the toggle
    // can't live in a group header until rokkit stops disabling them).
    // Intake (the front door) lives in the project window now — a chunk of work
    // always starts IN a project, so it's a per-project section, not observatory-wide.
    link("家", "Today", "/"),
    link("場", "Projects", "/projects", { badge: projectCount }),
    // Needs you — the daily payoff: everything with a pending decision.
    {
      text: "Needs you",
      children: [
        link("今", "Insights", "/insights"),
        link("覚", "Memories", "/learnings"),
        link("果", "Impact", "/impact"),
        link("巻", "Traceability", "/traceability"),
        link("贈", "Upgrades", "/upgrades"),
      ],
    },
  ];

  if (!focus) {
    // Review & diagnostics — reached periodically, hidden in Focus.
    entries.push({
      text: "Review",
      children: [
        link("録", "Sessions", "/sessions"),
        link("庫", "Libraries", "/libraries"),
        link("統", "Consolidation", "/consolidation"),
        link("具", "Instruments", "/instruments"),
        link("診", "Logs", "/activity-logs"),
        link("結", "Dōjō", "/dojo/connections"),
        link("群", "Sharing", "/dojo/sharing"),
        link("送", "Share review", "/share-review"),
      ],
    });
    // Settings — visited when something needs changing, hidden in Focus.
    entries.push({ type: "separator" });
    entries.push(link("調", "Settings", "/settings"));
  }

  return entries;
}

/** Flatten every link href from the full rail (focus-independent). */
function allHrefs(): string[] {
  const out: string[] = [];
  for (const e of buildNavItems({ focus: false })) {
    if ("href" in e) out.push(e.href);
    else if ("children" in e) for (const c of e.children) out.push(c.href);
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

  let best = "";
  for (const href of hrefs) {
    if (href === "/") continue;
    if (pathname.startsWith(href + "/") && href.length > best.length)
      best = href;
  }
  return best;
}
