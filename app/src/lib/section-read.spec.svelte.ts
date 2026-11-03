import { describe, it, expect } from 'vitest';
import { SectionRead } from './section-read.svelte.js';

// A Settings section's one read (Providers, Libraries): loading, then the data
// or why not — state, not component locals (frontend guidelines §2).
describe('SectionRead', () => {
  it('loading until the read settles, then no notice', async () => {
    const r = new SectionRead("Couldn't load routers");
    expect(r.loading).toBe(true);
    await r.run(async () => {});
    expect([r.loading, r.error, r.notice]).toEqual([false, null, null]);
  });

  it('a failed read is a fault Notice: what failed, then why', async () => {
    const r = new SectionRead("Couldn't load routers");
    await r.run(async () => { throw new Error('the daemon did not answer'); });
    expect(r.loading).toBe(false);
    expect(r.notice).toEqual({ tone: 'fault', title: "Couldn't load routers", detail: 'the daemon did not answer', remedy: 'Retry. If it keeps failing, reload the page.' });
  });

  it('a non-Error rejection is still said', async () => {
    const r = new SectionRead('x');
    await r.run(async () => { throw 'boom'; });
    expect(r.error).toBe('boom');
  });

  it('a retry clears the last failure first', async () => {
    const r = new SectionRead('x');
    await r.run(async () => { throw new Error('first'); });
    await r.run(async () => {});
    expect(r.notice).toBeNull();
  });
});
