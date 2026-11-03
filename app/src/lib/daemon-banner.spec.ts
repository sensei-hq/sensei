import { describe, it, expect } from 'vitest';
import { daemonBanner } from './daemon-banner';

// The app-wide strip, shown only while something is wrong with the daemon. It
// renders through the shared Banner (@sensei/ui); this is what it says.

describe('daemonBanner', () => {
  it('a degraded database: reconnecting, and where to see what is down', () => {
    expect(daemonBanner('degraded', null)).toEqual({
      tone: 'refusal',
      title: 'Reconnecting to the database',
      detail: 'Some data may be briefly unavailable while the daemon recovers.',
      action: { label: 'Open Health', href: '/health', name: 'health' },
    });
  });

  it('says when the settings could not be read, and that the last ones are kept', () => {
    expect(daemonBanner('full', 'Load failed')).toEqual({
      tone: 'refusal',
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
