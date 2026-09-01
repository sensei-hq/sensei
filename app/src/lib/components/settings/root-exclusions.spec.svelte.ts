// Per-root scan exclusions — what a user may type, and what it means.
//
// The mechanism already existed end to end (`folders_to_watch.excluded`,
// `PUT /api/scan/roots/{id}`, `RootWatcher::should_watch_path`) with no way to
// reach it: the Roots screen created every root with `excluded: []` and offered
// no editor. So this covers the translation from what someone types to what the
// watcher matches — the part that was never written.
//
// The measured case it exists for: `find-me-board` carries 1,230 folders, 1,211
// of them under `docs/proposal/deck-node` — an unpacked Node source tarball
// (openssl arch trees), with no `node_modules` anywhere, so nothing in the
// static EXCLUDE_DIRS list touches it.
import { describe, it, expect, vi } from 'vitest';
import {
  normaliseExclusion,
  describeExclusion,
  withExclusion,
  withoutExclusion,
  RootExclusions,
} from './root-exclusions.svelte.js';

const ROOT = '/Users/Jerry/Work';

describe('normaliseExclusion', () => {
  // The STORED form is RELATIVE to the root. `pg_store::folders::resolve_exclusion`
  // joins each entry onto the root exactly once, for both the live watcher and the
  // prune; existing live data agrees (`["Code"]` on the Developer root).
  //
  // Storing an ABSOLUTE path is therefore a silent no-op: the resolver joins it
  // anyway, producing `/Users/Jerry/Work/Users/Jerry/Work/…`, which matches
  // nothing. That is not hypothetical — it is what this control did on its first
  // live use, and the endpoint answered `{"ok":true,"prunedFolders":0}` with 1,212
  // folders sitting under the subtree.
  it('converts a pasted absolute path to root-relative', () => {
    expect(normaliseExclusion('/Users/Jerry/Work/find-me-board/docs/proposal/deck-node', ROOT))
      .toEqual({ ok: true, value: 'find-me-board/docs/proposal/deck-node' });
  });

  it('strips a trailing slash', () => {
    expect(normaliseExclusion('/Users/Jerry/Work/vendor/', ROOT)).toEqual({
      ok: true,
      value: 'vendor',
    });
  });

  it('expands ~ and still stores relative', () => {
    expect(normaliseExclusion('~/Work/vendor', ROOT).value).toBe('vendor');
  });

  it('REFUSES an absolute path outside the root instead of storing a no-op', () => {
    // It cannot be made relative to this root, so it could never match.
    const res = normaliseExclusion('/Users/Jerry/Developer/other', ROOT);
    expect(res.ok).toBe(false);
    expect(res.error).toContain('/Users/Jerry/Work');
  });

  it('keeps a bare name as a segment match', () => {
    expect(normaliseExclusion('node_modules', ROOT)).toEqual({
      ok: true,
      value: 'node_modules',
    });
  });

  it('keeps an already-relative path as typed', () => {
    expect(normaliseExclusion('find-me-board/docs', ROOT).value).toBe('find-me-board/docs');
  });

  it('refuses an empty or whitespace-only entry', () => {
    expect(normaliseExclusion('   ', ROOT).ok).toBe(false);
    expect(normaliseExclusion('', ROOT).ok).toBe(false);
  });
});

describe('describeExclusion', () => {
  it('says a single NAME matches everywhere, which is the surprising case', () => {
    // A one-segment entry is compared as `/name/` anywhere under the root; a
    // multi-segment one is a subtree. They look almost identical typed, and
    // excluding `docs` to stop one vendored tree would drop every docs folder.
    expect(describeExclusion('docs', ROOT)).toContain('every folder named');
  });

  it('says a multi-segment path is one subtree', () => {
    // Matched on the distinguishing PHRASE, not the substring "every" — the
    // subtree wording legitimately contains "everything under it".
    const text = describeExclusion('find-me-board/docs', ROOT);
    expect(text).toContain('find-me-board/docs');
    expect(text).not.toContain('every folder named');
  });
});

describe('withExclusion / withoutExclusion', () => {
  it('adds without duplicating', () => {
    expect(withExclusion(['a'], 'a')).toEqual(['a']);
    expect(withExclusion(['a'], 'b')).toEqual(['a', 'b']);
  });

  it('removes exactly one entry and leaves the rest', () => {
    expect(withoutExclusion(['a', 'b'], 'a')).toEqual(['b']);
    expect(withoutExclusion(['a', 'b'], 'zz')).toEqual(['a', 'b']);
  });

  it('does not mutate its input', () => {
    const before = ['a'];
    withExclusion(before, 'b');
    expect(before).toEqual(['a']);
  });
});

describe('RootExclusions', () => {
  const api = (over: Partial<Record<string, unknown>> = {}) => ({
    updateWatchRoot: vi
      .fn()
      .mockResolvedValue({ ok: true, data: { excluded: ['vendor'] } }),
    ...over,
  });

  it('sends the whole list, because the endpoint is a full replace', async () => {
    const a = api();
    const s = new RootExclusions('root-1', ROOT, ['node_modules'], a as never);
    await s.add('/Users/Jerry/Work/vendor');
    expect(a.updateWatchRoot).toHaveBeenCalledWith('root-1', ['node_modules', 'vendor']);
  });

  it('adopts the list the daemon echoes back, not the one it sent', async () => {
    const a = api();
    const s = new RootExclusions('root-1', ROOT, ['node_modules'], a as never);
    await s.add('/Users/Jerry/Work/vendor');
    expect(s.excluded).toEqual(['vendor']);
  });

  it('refuses a path outside the root WITHOUT calling the daemon', async () => {
    const a = api();
    const s = new RootExclusions('root-1', ROOT, [], a as never);
    const ok = await s.add('/somewhere/else');
    expect(ok).toBe(false);
    expect(a.updateWatchRoot).not.toHaveBeenCalled();
    expect(s.error).toContain('/Users/Jerry/Work');
  });

  it('leaves the list untouched when the write fails', async () => {
    // Same discipline as the metric toggle: the control must keep showing what is
    // stored, not what someone asked for. An exclusion that appears to be set but
    // is not means the sweep continues while the screen says it stopped.
    const a = api({
      updateWatchRoot: vi
        .fn()
        .mockResolvedValue({ ok: false, error: { status: 500, message: 'boom' } }),
    });
    const s = new RootExclusions('root-1', ROOT, ['node_modules'], a as never);
    await s.add('/Users/Jerry/Work/vendor');
    expect(s.excluded).toEqual(['node_modules']);
    expect(s.error).toBe('boom');
  });

  it('will not start a second write while one is in flight', async () => {
    let release: (v: unknown) => void = () => {};
    const a = api({ updateWatchRoot: vi.fn().mockReturnValue(new Promise((r) => (release = r))) });
    const s = new RootExclusions('root-1', ROOT, [], a as never);
    const first = s.add('/Users/Jerry/Work/a');
    await s.add('/Users/Jerry/Work/b');
    expect(a.updateWatchRoot).toHaveBeenCalledTimes(1);
    release({ ok: true, data: { excluded: [] } });
    await first;
  });

  it('removes an entry through the same full-replace path', async () => {
    const a = api({
      updateWatchRoot: vi.fn().mockResolvedValue({ ok: true, data: { excluded: ['b'] } }),
    });
    const s = new RootExclusions('root-1', ROOT, ['a', 'b'], a as never);
    await s.remove('a');
    expect(a.updateWatchRoot).toHaveBeenCalledWith('root-1', ['b']);
    expect(s.excluded).toEqual(['b']);
  });
});
