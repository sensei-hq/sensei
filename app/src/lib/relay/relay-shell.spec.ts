import { describe, it, expect, vi } from 'vitest';
import { toRelayShell, type RelayShellActions } from './relay-shell.js';
import {
  archive,
  calm,
  crewNoRun,
  dojoLoaded,
  severalCrews,
  error,
  filteredWorking,
  loadedCrewComposing,
  loadedNow,
  loadedQueue,
  loading,
  mainWindow,
  noRun,
  taskFromQueue,
  unreported,
} from './relay-shell.fixtures.js';

// The Relay shell as data (CONTRACT §1a, §1b, §1c-0, §1f, §1g; COMPONENTS §2, §4):
// a pure selector from the page's state to what ShellScreen draws. Rules the
// selector owns are pinned here, not in the component.

const actions = (): RelayShellActions => ({ view: vi.fn(), compose: vi.fn(), filter: vi.fn(), narrow: vi.fn() });
const ids = (items: Array<Record<string, unknown>>) => items.map((i) => (i.type === 'separator' ? '|' : String(i.id)));
const item = (items: Array<Record<string, unknown>>, id: string) => items.find((i) => i.id === id) as Record<string, unknown>;

describe('toRelayShell · toolbar', () => {
  it('loaded: the three views, a divider, Message the crew, the info toggle — in that order', () => {
    const shell = toRelayShell(loadedNow(), actions());
    expect(ids(shell.toolbar)).toEqual(['now', 'queue', 'crew', '|', 'message', 'info']);
  });

  it('a view button is a view, Message and Info are toggles: rokkit draws the 36px framed button from the kind (#420)', () => {
    const shell = toRelayShell(loadedNow(), actions());
    const now = item(shell.toolbar, 'now');
    expect(now.icon).toBe('i-solar-pulse-linear');
    expect(item(shell.toolbar, 'queue').icon).toBe('i-solar-layers-linear');
    expect(item(shell.toolbar, 'crew').icon).toBe('i-solar-users-group-rounded-linear');
    expect(item(shell.toolbar, 'message').icon).toBe('i-solar-chat-round-line-linear');
    expect(item(shell.toolbar, 'info').icon).toBe('i-solar-info-circle-linear');
    expect(now.active).toBe(true);
    expect(item(shell.toolbar, 'queue').active).toBe(false);
    expect(['now', 'queue', 'crew'].map((id) => item(shell.toolbar, id).kind)).toEqual(['view', 'view', 'view']);
    expect(['message', 'info'].map((id) => item(shell.toolbar, id).kind)).toEqual(['toggle', 'toggle']);
    // The look is the theme's: no class or hook of our own rides on a button.
    for (const id of ['now', 'queue', 'crew', 'message', 'info']) expect(item(shell.toolbar, id).attrs, id).toBeUndefined();
  });

  it('Queue and Crew carry their counts (rokkit draws the badge and names it "Queue · 7"); Now never does', () => {
    const shell = toRelayShell(loadedNow(), actions());
    expect(item(shell.toolbar, 'queue')).toMatchObject({ label: 'Queue', count: 7 });
    expect(item(shell.toolbar, 'crew')).toMatchObject({ label: 'Crew', count: 7 });
    expect(item(shell.toolbar, 'now')).toMatchObject({ label: 'Now', count: 0 });
  });

  it('a count of zero draws no badge', () => {
    const shell = toRelayShell(noRun(), actions());
    expect(item(shell.toolbar, 'crew')).toMatchObject({ label: 'Crew', count: 0 });
  });

  it('pressing a view asks the page for that view; Message toggles the compose band', () => {
    const a = actions();
    const shell = toRelayShell(loadedNow(), a);
    (item(shell.toolbar, 'queue').onclick as () => void)();
    expect(a.view).toHaveBeenCalledWith('queue');
    (item(shell.toolbar, 'message').onclick as () => void)();
    expect(a.compose).toHaveBeenCalledTimes(1);
  });

  it('a task opened from the Queue keeps Queue pressed; a past run keeps Crew pressed', () => {
    expect(item(toRelayShell(taskFromQueue(), actions()).toolbar, 'queue').active).toBe(true);
    expect(item(toRelayShell(taskFromQueue(), actions()).toolbar, 'now').active).toBe(false);
    expect(item(toRelayShell(archive(), actions()).toolbar, 'crew').active).toBe(true);
  });

  it('Message is pressed while the compose band is open', () => {
    expect(item(toRelayShell(loadedCrewComposing(), actions()).toolbar, 'message').active).toBe(true);
    expect(toRelayShell(loadedCrewComposing(), actions()).compose).toBe(true);
  });

  it('while reading, and after a failed read, only the info toggle stays: no view buttons, no Message', () => {
    expect(ids(toRelayShell(loading(), actions()).toolbar)).toEqual(['info']);
    expect(ids(toRelayShell(error(), actions()).toolbar)).toEqual(['info']);
  });

  it('with no run the views stay but Message does not (nobody to message)', () => {
    expect(ids(toRelayShell(noRun(), actions()).toolbar)).toEqual(['now', 'queue', 'crew', '|', 'info']);
  });

  it('the info toggle is a shell toggle of the info band, with its own icon and label', () => {
    const info = item(toRelayShell(loadedNow(), actions()).toolbar, 'info');
    expect(info.type).toBe('toggle');
    expect(info.value).toBe('info');
    expect(info.label).toBe('Hide info');
    expect(item(toRelayShell({ ...loadedNow(), info: false }, actions()).toolbar, 'info').label).toBe('Show info');
  });
});

describe('toRelayShell · view, context, info band', () => {
  it('with no run, the Crew view is the landing unless the Queue was asked for', () => {
    expect(toRelayShell(noRun(), actions()).view).toBe('crew');
    expect(item(toRelayShell(noRun(), actions()).toolbar, 'crew').active).toBe(true);
    expect(toRelayShell({ ...noRun(), view: 'queue' }, actions()).view).toBe('queue');
  });

  it('the context names the workspace and the run; before a run, the crew (or how many); none → "no run"', () => {
    expect(toRelayShell(loadedNow(), actions()).context).toBe('sensei · run 7f3a');
    expect(toRelayShell(crewNoRun(), actions()).context).toBe('sensei · crew c7f3');
    expect(toRelayShell(severalCrews(), actions()).context).toBe('sensei · 2 crews');
    expect(toRelayShell(noRun(), actions()).context).toBe('sensei · no run');
    expect(toRelayShell(dojoLoaded(), actions()).context).toBe('from your desktop · run 7f3a');
    // A read that failed before anything was known says so (D83), never "no run".
    expect(toRelayShell({ ...error(), run: null, crew: null, crews: 0 }, actions()).context).toBe('sensei · crew not reported');
  });

  it('Message the crew needs a crew, not a run: lanes register before a sprint starts', () => {
    expect(ids(toRelayShell(crewNoRun(), actions()).toolbar)).toContain('message');
  });

  it('loaded with a run: framing, the three steps (one line each, no eyebrow), and Today with the count', () => {
    const info = toRelayShell(loadedNow(), actions()).info;
    expect(info.map((c) => c.label)).toEqual(['Relay', '', 'Today']);
    expect(info[0].title).toBe('Your crew of assistant sessions');
    expect(info[1].steps?.map((s) => s.kanji)).toEqual(['問', '手', '史']);
    // The mockup's step is a mark and one regular line: the line is the text, there is no title.
    expect(info[1].steps?.map((s) => s.title)).toEqual(['', '', '']);
    expect(info[1].steps?.[2].text).toBe('Click any task for its history.');
    expect(info[2].value).toBe(5);
    expect(info[2].text).toContain('1 of 5 passed review first time');
  });

  it('while reading or after a failure the band stays whole and Today reads "not reported"', () => {
    for (const input of [loading(), error()]) {
      const info = toRelayShell(input, actions()).info;
      expect(info.map((c) => c.label)).toEqual(['Relay', '', 'Today']);
      expect(info[2].value).toBe('not reported');
    }
  });

  it('with no run the band is the framing and what to do (a display title, no eyebrow), pinned open', () => {
    const shell = toRelayShell(noRun(), actions());
    expect(shell.info.map((c) => c.label)).toEqual(['Relay', '']);
    expect(shell.info[1].title).toBe('No crew yet');
    expect(shell.info[1].text).toMatch(/Set it up below/);
    expect(shell.pinned).toEqual(['info']);
    expect(toRelayShell(loadedNow(), actions()).pinned).toEqual([]);
  });
});

describe('toRelayShell · stats', () => {
  it('four cards on Now: Working · Waiting · Blocked · Completed today, each a filter', () => {
    const stats = toRelayShell(loadedNow(), actions()).stats;
    expect(stats?.map((s) => s.id)).toEqual(['working', 'waiting', 'blocked', 'done']);
    expect(stats?.map((s) => s.kanji)).toEqual(['進', '待', '止', '済']);
    expect(stats?.map((s) => s.value)).toEqual([1, 4, 2, 5]);
    expect(stats?.map((s) => s.filter)).toEqual([
      { key: 'show', value: 'working' },
      { key: 'show', value: 'waiting' },
      { key: 'show', value: 'blocked' },
      { key: 'show', value: 'done' },
    ]);
  });

  it('Waiting warns past the threshold and says how old; Blocked is danger only when something is blocked', () => {
    const hot = toRelayShell(loadedNow(), actions()).stats!;
    expect(hot[1].tone).toBe('warning');
    expect(hot[1].description).toBe('oldest 52 min — too long');
    expect(hot[2].tone).toBe('danger');
    expect(hot[3].tone).toBe('success');
    expect(hot[3].description).toBe('1 of 5 first-pass');
    const quiet = toRelayShell(calm(), actions()).stats!;
    expect(quiet[1].tone).toBe('ink');
    expect(quiet[1].description).toBe('oldest 4 min');
    expect(quiet[2].tone).toBe('ink');
  });

  it('the stats band shows only on Now; it reads "still reading" while loading and is absent without a crew', () => {
    expect(toRelayShell(loadedQueue(), actions()).stats).toBeUndefined();
    expect(toRelayShell(loading(), actions()).stats).toBeNull();
    expect(toRelayShell(noRun(), actions()).stats).toBeUndefined();
    // A crew with no sprint yet still works, waits and blocks: the band shows.
    expect(toRelayShell(crewNoRun(), actions()).stats?.map((s) => s.value)).toEqual([0, 2, 0, 0]);
  });

  it('the active filter is the shell filter state the stat band reads', () => {
    expect(toRelayShell(filteredWorking(), actions()).filter).toEqual({ values: { show: 'working' }, stat: 'working' });
    expect(toRelayShell(loadedNow(), actions()).filter).toEqual({ values: {}, stat: null });
  });
});

describe('toRelayShell · the filter band (§1a, #384)', () => {
  type Element = { scope?: string; label?: string; props?: { options?: Array<{ value: string; label: string }>; placeholder?: string } };
  const layoutOf = (shell: ReturnType<typeof toRelayShell>) => (shell.filters?.layout as { elements: Element[] }).elements;

  it('the workspace window has no band: it is already narrowed to its workspace', () => {
    expect(toRelayShell(loadedNow(), actions()).filters).toBeUndefined();
  });

  it('the main window draws repository and workspace as a rokkit schema form, not hand-made selects', () => {
    const shell = toRelayShell(mainWindow(), actions());
    expect(shell.filters?.schema).toEqual({
      type: 'object',
      properties: { repo: { type: 'string' }, workspace: { type: 'string' } },
    });
    const [repo, workspace] = layoutOf(shell);
    expect(repo.scope).toBe('#/repo');
    expect(repo.label).toBe('Repository');
    // "All" is the empty value: the select shows it as its placeholder.
    expect(repo.props?.options).toEqual([
      { value: '', label: 'All repositories' },
      { value: 'github.com/sensei-hq/sensei', label: 'sensei-hq/sensei' },
      { value: 'github.com/jerrythomas/scratch', label: 'jerrythomas/scratch' },
    ]);
    expect(repo.props?.placeholder).toBe('All repositories');
    expect(workspace.scope).toBe('#/workspace');
    expect(workspace.props?.options).toEqual([
      { value: '', label: 'All workspaces' },
      { value: 'w1', label: 'Sensei' },
    ]);
  });

  it('the filter values carry the URL’s narrowing beside the stat, one source of truth', () => {
    expect(toRelayShell(mainWindow(), actions()).filter).toEqual({ values: { repo: '', workspace: 'w1' }, stat: null });
    expect(toRelayShell({ ...mainWindow(), filter: 'blocked' }, actions()).filter).toEqual({
      values: { repo: '', workspace: 'w1', show: 'blocked' },
      stat: 'blocked',
    });
  });

  it('a change from the band narrows by the one key that moved; a stat change sets the stat filter', () => {
    const a = actions();
    const shell = toRelayShell(mainWindow(), a);
    shell.onfilter({ values: { repo: 'github.com/jerrythomas/scratch', workspace: 'w1' }, stat: null });
    expect(a.narrow).toHaveBeenCalledWith('repo', 'github.com/jerrythomas/scratch');
    expect(a.narrow).toHaveBeenCalledTimes(1);
    expect(a.filter).not.toHaveBeenCalled();

    shell.onfilter({ values: { repo: '', workspace: '' }, stat: null });
    expect(a.narrow).toHaveBeenLastCalledWith('workspace', '');

    shell.onfilter({ values: { repo: '', workspace: 'w1', show: 'waiting' }, stat: 'waiting' });
    expect(a.filter).toHaveBeenCalledWith('waiting');
    expect(a.narrow).toHaveBeenCalledTimes(2);
  });

  it('a cleared value (null from the select) narrows to all', () => {
    const a = actions();
    toRelayShell(mainWindow(), a).onfilter({ values: { repo: '', workspace: null }, stat: null });
    expect(a.narrow).toHaveBeenCalledWith('workspace', '');
  });

  it('clearing the stat clears the stat filter; the workspace window only ever moves the stat', () => {
    const a = actions();
    toRelayShell({ ...mainWindow(), filter: 'working' }, a).onfilter({ values: { repo: '', workspace: 'w1' }, stat: null });
    expect(a.filter).toHaveBeenCalledWith(null);
    expect(a.narrow).not.toHaveBeenCalled();

    const b = actions();
    toRelayShell(loadedNow(), b).onfilter({ values: { show: 'done' }, stat: 'done' });
    expect(b.filter).toHaveBeenCalledWith('done');
    expect(b.narrow).not.toHaveBeenCalled();
  });
});

describe('toRelayShell · footer', () => {
  it('desktop: the build lease, free disk and free swap, each with a tone', () => {
    const footer = toRelayShell(loadedNow(), actions()).footer;
    expect(footer).toEqual([
      { tone: 'accent', text: 'Rust build: cobalt-lyra (4 min)' },
      { tone: 'success', text: 'Disk 46 GB free' },
      { tone: 'warning', text: 'Swap 1.2 GB free (floor 1 GB)' },
    ]);
  });

  it('a number the daemon did not report reads "not reported", never 0', () => {
    const footer = toRelayShell(unreported(), actions()).footer;
    expect(footer.map((f) => f.text)).toEqual(['No build running', 'Disk not reported', 'Swap not reported']);
    expect(footer.map((f) => f.tone)).toEqual(['muted', 'muted', 'muted']);
  });

  it('no read at all (reading, or it failed) keeps the footer whole: each resource "not reported" (§1f, D83)', () => {
    for (const input of [{ ...loading(), resources: null }, { ...error(), resources: null }]) {
      const footer = toRelayShell(input, actions()).footer;
      expect(footer.map((f) => f.text)).toEqual(['Build not reported', 'Disk not reported', 'Swap not reported']);
      expect(footer.map((f) => f.tone)).toEqual(['muted', 'muted', 'muted']);
    }
  });

  it('the dōjō: when it was last updated from the desktop, and what is shared', () => {
    expect(toRelayShell(dojoLoaded(), actions()).footer).toEqual([
      { tone: 'success', text: 'From your desktop · 1 min ago' },
      { tone: 'success', text: 'Details shared' },
    ]);
  });
});
