// The Relay shell as data (CONTRACT §1a, §1b, §1c-0, §1f, §1g; COMPONENTS §2,
// §4): one pure selector from the page's state to what `RelayShell` hands
// `ShellScreen` — the toolbar (three views, a divider, Message the crew, the
// info toggle), the context line, the info band per load state, the stats
// band on Now, the footer. Every rule the contract gives the shell lives here,
// unit-tested on data; the component only draws the result.

import type { FilterState, InfoColumn, ShellStat } from '@rokkit/app';
import type { ToolbarItem } from '@rokkit/ui';
import { toolbarBadgeAfterClass, toolbarIconClass, toolbarItemClass, toolbarLabel } from '@sensei/ui';
import { KANJI_ICONS } from '$lib/glyph-vocabulary';

export type RelaySurface = 'desktop' | 'dojo';
export type RelayLoad = 'loading' | 'error' | 'empty' | 'loaded';
export type RelayView = 'now' | 'queue' | 'crew' | 'archive' | 'task';
export type RelayStatFilter = 'working' | 'waiting' | 'blocked' | 'done';
export type RelayFooterTone = 'accent' | 'success' | 'warning' | 'danger' | 'muted';

export interface RelayShellInput {
  surface: RelaySurface;
  load: RelayLoad;
  view: RelayView;
  /** Where the task or archive view came from — it keeps that button pressed. */
  from: 'now' | 'queue' | 'archive' | null;
  workspace: string;
  run: { id: string } | null;
  counts: { queue: number; crew: number };
  today: { done: number; firstPass: string } | null;
  stats: {
    working: number;
    waiting: { count: number; oldestMin: number | null };
    blocked: number;
    done: number;
    firstPass: string;
  } | null;
  filter: RelayStatFilter | null;
  compose: boolean;
  /** Whether the info band is open (the toggle's label follows it). */
  info: boolean;
  resources: {
    build: { resource: string; lane: string; minutes: number } | null;
    disk_free_gb: number | null;
    swap_free_gb: number | null;
    swap_floor_gb: number | null;
  } | null;
  freshness: { updated: string; shared: boolean } | null;
  /** lanes.md §15: a wait at or past it is too long. */
  threshold: number;
}

export interface RelayShellActions {
  view(id: RelayView): void;
  compose(): void;
  filter(id: RelayStatFilter | null): void;
}

export interface RelayFooterItem {
  tone: RelayFooterTone;
  text: string;
}

export interface RelayShell {
  /** The view drawn — with no run, Crew unless Queue was asked for. */
  view: RelayView;
  context: string;
  toolbar: ToolbarItem[];
  info: InfoColumn[];
  /** Bands held open whatever the store remembers. */
  pinned: Array<'info'>;
  /** `undefined` = no band; `null` = still reading. */
  stats: ShellStat[] | null | undefined;
  filter: FilterState;
  compose: boolean;
  footer: RelayFooterItem[];
}

// Solar Linear (user, 2026-11-10): pulse · layers · users-group-rounded ·
// chat-round-line · info-circle. Literal, so UnoCSS generates them.
const VIEW_ICONS: Record<'now' | 'queue' | 'crew', string> = {
  now: 'i-solar-pulse-linear',
  queue: 'i-solar-layers-linear',
  crew: 'i-solar-users-group-rounded-linear',
};
const MESSAGE_ICON = 'i-solar-chat-round-line-linear';
const INFO_ICON = 'i-solar-info-circle-linear';
const STAT_ICONS: Record<RelayStatFilter, string> = {
  working: 'i-solar-play-circle-linear',
  waiting: 'i-solar-clock-circle-linear',
  blocked: 'i-solar-close-circle-linear',
  done: 'i-solar-check-circle-linear',
};

const FRAMING: InfoColumn = {
  label: 'Relay',
  title: 'Your crew of assistant sessions',
  text: 'Each lane is one assistant working one task. Reviews come back before new work. When one needs you, it shows at the top.',
};
// The mockup's middle column is three marks with one regular line each and no
// eyebrow; rokkit draws a step's `title` bold and its `text` regular, and the
// label always — so the line is the text and the label is empty.
const HOW_TO_READ: InfoColumn = {
  label: '',
  steps: [
    { kanji: '問', icon: KANJI_ICONS['問'], title: '', text: 'What needs you shows first: decisions, and items to triage.' },
    { kanji: '手', icon: KANJI_ICONS['手'], title: '', text: 'Each lane works one of your backlog tasks at a time; a lane left waiting too long shows here.' },
    { kanji: '史', icon: KANJI_ICONS['史'], title: '', text: 'Click any task for its history.' },
  ],
};
const NO_RUN: InfoColumn = {
  label: '',
  title: 'No crew yet',
  text: 'Set it up below: start one session as the coordinator, then one or more contributors. Any mix of assistants, and assistants inside an editor, works.',
};

/** The button a view, a task or an archive keeps pressed. */
function pressedView(input: RelayShellInput): RelayView {
  if (input.load === 'empty') return input.view === 'queue' ? 'queue' : 'crew';
  if (input.view === 'task') return input.from === 'archive' ? 'crew' : (input.from ?? 'now');
  if (input.view === 'archive') return 'crew';
  return input.view;
}

/** A 36px icon button: the shell toolbar renders the button and hangs `attrs` on
 *  it, so the look (and the badge, from `data-count`) rides the class. */
function iconButton(
  id: string,
  label: string,
  icon: string,
  kind: 'view' | 'toggle',
  pressed: boolean,
  count: number,
  onclick: () => void,
): ToolbarItem {
  const classes = [toolbarItemClass(pressed), toolbarIconClass(kind, pressed), count > 0 ? toolbarBadgeAfterClass() : ''];
  const attrs: Record<string, string> = { class: classes.filter(Boolean).join(' ') };
  if (kind === 'view') attrs['data-relay-view'] = id;
  if (count > 0) attrs['data-count'] = String(count);
  return { id, value: id, type: 'button', label: toolbarLabel(label, count), icon, active: pressed, attrs, onclick };
}

function toolbar(input: RelayShellInput, pressed: RelayView, actions: RelayShellActions): ToolbarItem[] {
  const read = input.load === 'loaded' || input.load === 'empty';
  const info: ToolbarItem = {
    id: 'info',
    value: 'info',
    type: 'toggle',
    icon: INFO_ICON,
    label: input.info ? 'Hide info' : 'Show info',
    attrs: { class: [toolbarItemClass(input.info), toolbarIconClass('toggle', input.info)].join(' ') },
  };
  if (!read) return [info];
  const views: ToolbarItem[] = [
    iconButton('now', 'Now', VIEW_ICONS.now, 'view', pressed === 'now', 0, () => actions.view('now')),
    iconButton('queue', 'Queue', VIEW_ICONS.queue, 'view', pressed === 'queue', input.counts.queue, () => actions.view('queue')),
    iconButton('crew', 'Crew', VIEW_ICONS.crew, 'view', pressed === 'crew', input.counts.crew, () => actions.view('crew')),
  ];
  const message = input.run
    ? [iconButton('message', 'Message the crew', MESSAGE_ICON, 'toggle', input.compose, 0, () => actions.compose())]
    : [];
  return [...views, { type: 'separator' }, ...message, info];
}

function infoBand(input: RelayShellInput): InfoColumn[] {
  if (input.load === 'empty') return [FRAMING, NO_RUN];
  const today: InfoColumn = input.today
    ? { label: 'Today', value: input.today.done, text: `completed · ${input.today.firstPass}` }
    : { label: 'Today', value: 'not reported', text: 'completed' };
  return [FRAMING, HOW_TO_READ, today];
}

function stats(input: RelayShellInput, pressed: RelayView): ShellStat[] | null | undefined {
  if (input.load === 'loading') return null;
  if (input.load !== 'loaded' || !input.run || pressed !== 'now') return undefined;
  const s = input.stats;
  if (!s) return null;
  const oldest = s.waiting.oldestMin;
  const over = s.waiting.count > 0 && oldest != null && oldest >= input.threshold;
  const waitingText =
    s.waiting.count === 0 ? 'no one waiting' : oldest == null ? 'oldest not reported' : `oldest ${oldest} min${over ? ' — too long' : ''}`;
  const card = (id: RelayStatFilter, kanji: string, title: string, value: number, description: string, tone: ShellStat['tone']): ShellStat => ({
    id,
    title,
    kanji,
    icon: STAT_ICONS[id],
    value,
    description,
    tone,
    filter: { key: 'show', value: id },
    attrs: { 'data-relay-stat': id },
  });
  return [
    // Working is accent on the mockup; rokkit's stat tones have no accent — ink until Design rules.
    card('working', '進', 'Working', s.working, 'on a task now', 'ink'),
    card('waiting', '待', 'Waiting', s.waiting.count, waitingText, over ? 'warning' : 'ink'),
    card('blocked', '止', 'Blocked', s.blocked, 'on you, or lease lost', s.blocked > 0 ? 'danger' : 'ink'),
    card('done', '済', 'Completed today', s.done, s.firstPass, 'success'),
  ];
}

/** "rust-build" → "Rust build". */
export const resourceName = (id: string): string => {
  const words = id.replace(/[-_]/g, ' ').trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
};

function footer(input: RelayShellInput): RelayFooterItem[] {
  if (input.surface === 'dojo') {
    if (!input.freshness) return [{ tone: 'muted', text: 'Not yet updated from your desktop' }];
    return [
      { tone: 'success', text: `From your desktop · ${input.freshness.updated}` },
      input.freshness.shared ? { tone: 'success', text: 'Details shared' } : { tone: 'muted', text: 'Headers and decisions only' },
    ];
  }
  const r = input.resources;
  if (!r) return [];
  const build: RelayFooterItem = r.build
    ? { tone: 'accent', text: `${resourceName(r.build.resource)}: ${r.build.lane} (${r.build.minutes} min)` }
    : { tone: 'muted', text: 'No build running' };
  const disk: RelayFooterItem =
    r.disk_free_gb == null ? { tone: 'muted', text: 'Disk not reported' } : { tone: 'success', text: `Disk ${r.disk_free_gb} GB free` };
  let swap: RelayFooterItem;
  if (r.swap_free_gb == null) swap = { tone: 'muted', text: 'Swap not reported' };
  else {
    const floor = r.swap_floor_gb;
    const tone: RelayFooterTone = floor == null ? 'success' : r.swap_free_gb < floor ? 'danger' : r.swap_free_gb < floor * 2 ? 'warning' : 'success';
    swap = { tone, text: floor == null ? `Swap ${r.swap_free_gb} GB free` : `Swap ${r.swap_free_gb} GB free (floor ${floor} GB)` };
  }
  return [build, disk, swap];
}

export function toRelayShell(input: RelayShellInput, actions: RelayShellActions): RelayShell {
  const pressed = pressedView(input);
  const view: RelayView = input.load === 'empty' ? pressed : input.view;
  const where = input.surface === 'dojo' ? 'from your desktop' : input.workspace;
  return {
    view,
    context: `${where} · ${input.run ? `run ${input.run.id}` : 'no run'}`,
    toolbar: toolbar(input, pressed, actions),
    info: infoBand(input),
    pinned: input.load === 'empty' ? ['info'] : [],
    stats: stats(input, pressed),
    filter: input.filter ? { values: { show: input.filter }, stat: input.filter } : { values: {}, stat: null },
    compose: input.compose && Boolean(input.run),
    footer: footer(input),
  };
}
