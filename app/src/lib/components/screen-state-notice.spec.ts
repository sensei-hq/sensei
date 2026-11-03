import { describe, it, expect } from 'vitest';
import { screenErrorNotice } from './screen-state-notice';

describe('screenErrorNotice', () => {
  it('a bare error string: the generic title, the message as the reason', () => {
    expect(screenErrorNotice('Failed to fetch')).toEqual({
      tone: 'fault', eyebrow: "couldn't load", title: "Couldn't load this", detail: 'Failed to fetch', remedy: undefined,
    });
  });

  it('no message at all: says what to check instead of an empty line', () => {
    const n = screenErrorNotice(null);
    expect(n.detail).toBeUndefined();
    expect(n.remedy).toMatch(/sensei daemon is running/);
  });

  it("a state module's notice wins whole, with the screen defaults filled in", () => {
    expect(screenErrorNotice('ignored', { title: "Couldn't load tickets", code: 'http_503' })).toEqual({
      tone: 'fault', eyebrow: "couldn't load", title: "Couldn't load tickets", code: 'http_503',
    });
  });

  it('a notice may set its own tone and eyebrow', () => {
    const n = screenErrorNotice(null, { tone: 'refusal', eyebrow: 'not allowed', title: 'Admins only' });
    expect(n.tone).toBe('refusal');
    expect(n.eyebrow).toBe('not allowed');
  });
});
