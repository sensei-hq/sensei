import { describe, it, expect } from 'vitest';
import { LibrariesState, type LibrariesApi } from './libraries-state.svelte.js';
import type { LibEntry } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const lib = (id: string, name = id) => ({ id, name, repoCount: 1, repos: [] }) as unknown as LibEntry;
const usage = [{ library_name: 'rokkit', folder: 'app', version_used: '1.0', import_count: 4 }];

const skills = [{ name: 'styling', focus: 'styling', body: '# styling', source: 'manifest' }];
const agents = [{ name: 'rokkit-styles-reviewer', focus: 'styles', body: '# review', source: 'manifest' }];
const docs = [{ title: 'Button', component: 'Button', content: '# Button' }];

const api = (over: Partial<LibrariesApi> = {}): LibrariesApi => ({
  tryGetLibs: async () => ok({ total: 2, libs: [lib('a', 'rokkit'), lib('b', 'zod')] }),
  tryGetLibraryUsage: async () => ok({ usage }),
  tryGetLibrarySkills: async () => ok(skills),
  tryGetLibraryAgents: async () => ok(agents),
  tryGetLibraryDocs: async () => ok(docs),
  ...over,
});

// #288: the libraries and a library's usage, or why they could not be read —
// not "No libraries indexed." and not a library that looks unused.
describe('LibrariesState', () => {
  it('loads every detected library', async () => {
    const s = new LibrariesState();
    await s.load(api());
    expect([s.loading, s.error, s.libs.map((l) => l.name)]).toEqual([false, null, ['rokkit', 'zod']]);
  });

  it('a failed list read is an error, not an empty index', async () => {
    const s = new LibrariesState();
    await s.load(api({ tryGetLibs: async () => down }));
    expect([s.loading, s.error, s.libs]).toEqual([false, 'Service Unavailable', []]);
  });

  it('selecting a library reads its usage', async () => {
    const s = new LibrariesState();
    await s.select(api(), lib('a', 'rokkit'));
    expect([s.selected?.id, s.usage, s.usageError, s.usageLoading]).toEqual(['a', usage, null, false]);
  });

  it('a failed usage read is said so — not a library that looks unused', async () => {
    const s = new LibrariesState();
    await s.select(api({ tryGetLibraryUsage: async () => down }), lib('a'));
    expect([s.usage, s.usageError]).toEqual([[], 'Service Unavailable']);
  });

  it('a stale usage answer for a previous selection is dropped', async () => {
    const s = new LibrariesState();
    let release!: () => void;
    const slow = api({ tryGetLibraryUsage: (id) => (id === 'a' ? new Promise((r) => (release = () => r(ok({ usage })))) : Promise.resolve(ok({ usage: [] }))) });
    const first = s.select(slow, lib('a'));
    await s.select(slow, lib('b'));
    release();
    await first;
    expect([s.selected?.id, s.usage]).toEqual(['b', []]);
  });

  // #341: what the library itself provides — its sensei.library.json docs,
  // skills and agents — read when it is selected, by its id: a name can be two
  // libraries (`uuid` is a crate and an npm package).
  it('selecting a library reads the docs, skills and agents it provides', async () => {
    const s = new LibrariesState();
    const asked: string[] = [];
    const ask = (ref: string) => (asked.push(ref), ok([]));
    await s.select(
      api({
        tryGetLibrarySkills: async (ref) => (ask(ref), ok(skills)),
        tryGetLibraryAgents: async (ref) => (ask(ref), ok(agents)),
        tryGetLibraryDocs: async (ref) => (ask(ref), ok(docs)),
      }),
      lib('a', 'rokkit'),
    );
    expect(asked).toEqual(['a', 'a', 'a']);
    expect([s.provided.skills, s.provided.agents, s.provided.docs, s.provided.error]).toEqual([skills, agents, docs, null]);
  });

  it('a failed read of what it provides is said so — not a library that provides nothing', async () => {
    const s = new LibrariesState();
    await s.select(api({ tryGetLibraryAgents: async () => down }), lib('a', 'rokkit'));
    expect([s.provided.skills, s.provided.agents, s.provided.docs, s.provided.error]).toEqual([[], [], [], 'Service Unavailable']);
  });

  it('a stale provided answer for a previous selection is dropped', async () => {
    const s = new LibrariesState();
    let release!: () => void;
    const slow = api({
      tryGetLibrarySkills: (ref) => (ref === 'a' ? new Promise((r) => (release = () => r(ok(skills)))) : Promise.resolve(ok([]))),
    });
    const first = s.select(slow, lib('a', 'rokkit'));
    await s.select(slow, lib('b', 'zod'));
    release();
    await first;
    expect([s.selected?.name, s.provided.skills]).toEqual(['zod', []]);
  });
});
