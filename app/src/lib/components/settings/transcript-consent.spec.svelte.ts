// Consent to read an assistant's conversation history (#218).
//
// Configuring an assistant writes hooks; it never asked whether sensei may read
// past conversations off disk, and the daemon read all six sources anyway. The
// daemon now reads only what was consented to. This controller is the switch,
// and it must never show a yes the daemon did not store.

import { describe, expect, it } from 'vitest';
import { TranscriptConsent, type TranscriptConsentApi } from './transcript-consent.svelte.js';
import type { TranscriptSourceConsent } from '$lib/types.js';

const rows = (): TranscriptSourceConsent[] => [
  { source: 'claude_code', label: 'Claude Code', consented: false },
  { source: 'zed', label: 'Zed', consented: true },
];

function api(over: Partial<TranscriptConsentApi> = {}) {
  const puts: Array<[string, boolean]> = [];
  const a: TranscriptConsentApi = {
    tryGetTranscriptConsent: async () => ({ ok: true, data: rows() }),
    trySetTranscriptConsent: async (source, consented) => {
      puts.push([source, consented]);
      return { ok: true, data: { ok: true, source, consented } };
    },
    ...over,
  };
  return { a, puts };
}

describe('TranscriptConsent', () => {
  it('lists every source with its state', async () => {
    const { a } = api();
    const c = new TranscriptConsent(a);
    await c.load();
    expect(c.sources.map((s) => [s.label, s.consented])).toEqual([
      ['Claude Code', false],
      ['Zed', true],
    ]);
  });

  // An unreadable consent is not "everything off" — that would look like a
  // choice the user made.
  it('a failed read is a failure, and offers no switches', async () => {
    const { a } = api({
      tryGetTranscriptConsent: async () => ({ ok: false, error: { status: 500, message: 'boom' } }),
    });
    const c = new TranscriptConsent(a);
    await c.load();
    expect(c.failure).toContain('500');
    expect(c.sources).toEqual([]);
  });

  it('a switch stores the opposite of what is shown, then shows what was stored', async () => {
    const { a, puts } = api();
    const c = new TranscriptConsent(a);
    await c.load();
    await c.toggle('claude_code');
    expect(puts).toEqual([['claude_code', true]]);
    expect(c.sources.find((s) => s.source === 'claude_code')?.consented).toBe(true);
  });

  it('turning one off shows it off', async () => {
    const { a, puts } = api();
    const c = new TranscriptConsent(a);
    await c.load();
    await c.toggle('zed');
    expect(puts).toEqual([['zed', false]]);
    expect(c.sources.find((s) => s.source === 'zed')?.consented).toBe(false);
  });

  // The screen never shows a yes the daemon did not store.
  it('a failed write leaves the switch where it was and says why', async () => {
    const { a } = api({
      trySetTranscriptConsent: async () => ({ ok: false, error: { status: 0, message: 'connection refused' } }),
    });
    const c = new TranscriptConsent(a);
    await c.load();
    await c.toggle('claude_code');
    expect(c.sources.find((s) => s.source === 'claude_code')?.consented).toBe(false);
    expect(c.failure).toMatch(/could not reach the daemon/i);
  });

  it('says in plain words what a yes means', () => {
    const { a } = api();
    expect(new TranscriptConsent(a).explanation).toBe(
      'Reads past conversations from disk and stores them in sensei’s local database. Off until you turn it on. Configuring an assistant does not turn this on.',
    );
  });
});
