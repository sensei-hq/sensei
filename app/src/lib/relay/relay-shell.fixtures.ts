// The states the Relay shell can be in (CONTRACT §1a, §1b, §1c-0, §1f, §1g),
// one input object each. The selector spec drives `toRelayShell` with them and
// the gallery renders `RelayShell` from them, so a fixture here is both a test
// case and a picture on the bench.

import { WAIT_THRESHOLD_MIN, type RelayShellInput } from './relay-shell.js';

const resources = {
  build: { resource: 'rust-build', lane: 'cobalt-lyra', minutes: 4 },
  disk_free_gb: 46,
  swap_free_gb: 1.2,
  swap_floor_gb: 1,
};

const base: RelayShellInput = {
  surface: 'desktop',
  load: 'loaded',
  view: 'now',
  from: null,
  workspace: 'sensei',
  run: { id: '7f3a' },
  crew: { id: 'c7f3' },
  crews: 1,
  counts: { queue: 7, crew: 7 },
  today: { done: 5, firstPass: '1 of 5 passed review first time.' },
  stats: { working: 1, waiting: { count: 4, oldestMin: 52 }, blocked: 2, done: 5, firstPass: '1 of 5 first-pass' },
  filter: null,
  compose: false,
  info: true,
  resources,
  freshness: null,
  threshold: WAIT_THRESHOLD_MIN,
  narrow: null,
};

const over = (patch: Partial<RelayShellInput>): RelayShellInput => ({ ...base, ...patch });

/** Loaded, on Now: every button, the stats band, the full info band. */
export const loadedNow = (): RelayShellInput => over({});
/** Queue pressed. */
export const loadedQueue = (): RelayShellInput => over({ view: 'queue' });
/** Crew pressed; a message being composed. */
export const loadedCrewComposing = (): RelayShellInput => over({ view: 'crew', compose: true });
/** A task opened from the queue: the Queue button stays pressed. */
export const taskFromQueue = (): RelayShellInput => over({ view: 'task', from: 'queue' });
/** A past run opened: the Crew button stays pressed. */
export const archive = (): RelayShellInput => over({ view: 'archive', from: 'archive' });
/** Working filter on: the stat is pressed. */
export const filteredWorking = (): RelayShellInput => over({ filter: 'working' });
/** Nothing waits long: Waiting reads in ink, not warning. */
export const calm = (): RelayShellInput =>
  over({ stats: { working: 3, waiting: { count: 1, oldestMin: 4 }, blocked: 0, done: 2, firstPass: '2 of 2 first-pass' } });
/** Still reading: the shell whole, no view buttons, no Message, stats "reading", Today not reported. */
export const loading = (): RelayShellInput => over({ load: 'loading', today: null, stats: null, counts: { queue: 0, crew: 0 } });
/** The read failed: as loading, and the body carries the Notice. */
export const error = (): RelayShellInput => over({ load: 'error', today: null, stats: null, counts: { queue: 0, crew: 0 } });
/** No run yet: the Crew view is the landing, the info band says what to do. */
export const noRun = (): RelayShellInput =>
  over({ load: 'empty', view: 'now', run: null, crew: null, crews: 0, counts: { queue: 7, crew: 0 }, today: null, stats: null });
/** A crew registered, no sprint started yet: Message works, the context names the crew. */
export const crewNoRun = (): RelayShellInput => over({ run: null, stats: { working: 0, waiting: { count: 2, oldestMin: null }, blocked: 0, done: 0, firstPass: 'no review reported yet' } });
/** The main window holds several crews: the context counts them. */
export const severalCrews = (): RelayShellInput => over({ run: null, crews: 2 });
/** The same shell on the dōjō: context and footer differ, nothing else. */
export const dojoLoaded = (): RelayShellInput =>
  over({ surface: 'dojo', resources: null, freshness: { updated: '1 min ago', shared: true } });
/** The main window: every crew, narrowed by repository or workspace through the filter band (§1a). */
export const mainWindow = (): RelayShellInput =>
  over({
    workspace: 'all workspaces',
    run: null,
    crews: 2,
    narrow: {
      repos: ['github.com/sensei-hq/sensei', 'github.com/jerrythomas/scratch'],
      workspaces: [{ id: 'w1', name: 'Sensei' }],
      repo: '',
      workspace: 'w1',
    },
  });
/** Numbers the daemon did not report read "not reported", never 0 (D83). */
export const unreported = (): RelayShellInput =>
  over({ resources: { build: null, disk_free_gb: null, swap_free_gb: null, swap_floor_gb: null } });

export const SHELL_FIXTURES: Array<{ name: string; note: string; input: () => RelayShellInput }> = [
  { name: 'loaded-now', note: 'loaded, on Now: five buttons, the stats band, the full info band', input: loadedNow },
  { name: 'loaded-queue', note: 'Queue pressed, with its count', input: loadedQueue },
  { name: 'loaded-crew-composing', note: 'Crew pressed; Message the crew open (compose band)', input: loadedCrewComposing },
  { name: 'task-from-queue', note: 'a task opened from the Queue: Queue stays pressed', input: taskFromQueue },
  { name: 'filtered-working', note: 'the Working stat pressed', input: filteredWorking },
  { name: 'calm', note: 'nothing waits past the threshold', input: calm },
  { name: 'loading', note: 'still reading: no view buttons, no Message, Today not reported', input: loading },
  { name: 'error', note: 'the read failed: the shell stays whole', input: error },
  { name: 'no-run', note: 'no run yet: Crew is the landing, the info band says what to do', input: noRun },
  { name: 'crew-no-run', note: 'a crew registered, no sprint yet: the context names the crew', input: crewNoRun },
  { name: 'dojo', note: 'the dōjō surface: context and footer', input: dojoLoaded },
  { name: 'main-window', note: 'the main window: repository and workspace filters as a schema form', input: mainWindow },
  { name: 'unreported', note: 'resources the daemon did not report', input: unreported },
];
