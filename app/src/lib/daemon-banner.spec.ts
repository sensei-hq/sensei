import { describe, it, expect } from 'vitest';
import { daemonBanner } from './daemon-banner';

// The app-wide strip, shown only while something is wrong with the daemon. It
// renders through the shared Banner (@sensei/ui); this is what it says.

describe('daemonBanner', () => {
  // Tones keep their meaning (L1, r1): a self-healing reconnect is a normal wait;
  // a settings read that failed is a fault. Neither is a refusal.
  it('a degraded database: a normal wait, and where to see what is down', () => {
    expect(daemonBanner('degraded', null)).toEqual({
      tone: 'normal',
      title: 'Reconnecting to the database',
      detail: 'Some data may be briefly unavailable while the daemon recovers.',
      action: { label: 'Open Health', href: '/health', name: 'health' },
    });
  });

  it('says when the settings could not be read, and that the last ones are kept', () => {
    expect(daemonBanner('full', 'Load failed')).toEqual({
      tone: 'fault',
      title: 'Could not read your settings from the daemon',
      detail: 'Load failed — showing the last ones; trying again.',
      action: { label: 'Open Health', href: '/health', name: 'health' },
    });
  });

  it('says nothing when all is well, or the mode is not known yet', () => {
    expect(daemonBanner('full', null)).toBeNull();
    expect(daemonBanner(undefined, null)).toBeNull();
  });
});
