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

  // 8.4: the two states the banner ignored — a dead daemon, and a first
  // install still building its database. Down is a fault (nothing heals it
  // but attention); provisioning is a wait (never a fault, never a refusal).
  it('a dead daemon names the outage, distinct from every other banner', () => {
    expect(daemonBanner(undefined, null, true)).toEqual({
      tone: 'fault',
      title: 'Daemon not answering',
      detail: 'The app shows the last known state. Check that the daemon is running, then open Health.',
      action: { label: 'Open Health', href: '/health', name: 'health' },
    });
  });

  it('a provisioning database is a normal wait, not a fault', () => {
    expect(daemonBanner('provisioning', null)).toEqual({
      tone: 'normal',
      title: 'Setting things up',
      detail: 'The database is still being created — the app fills in as it arrives.',
      action: { label: 'Open Health', href: '/health', name: 'health' },
    });
  });

  it('down wins over every other banner', () => {
    expect(daemonBanner('degraded', 'Load failed', true)?.title).toBe('Daemon not answering');
  });
});
