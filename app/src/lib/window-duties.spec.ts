import { describe, it, expect } from 'vitest';
import { windowDuties } from './window-duties';

// Every window runs the root layout. Settings became its own window
// (2026-10-27), and the review found it doubling the app's global work: a
// second credential-renewal poller (two browser tabs for one renewal), menu
// events acted on twice, the identity overlay opening in Settings.
describe('windowDuties', () => {
  it('the main window renews credentials, announces identities that need you, owns the menu, and goes anywhere', () => {
    const d = windowDuties('main');
    expect([d.renewsCredentials, d.announcesIdentities, d.handlesMenu]).toEqual([true, true, true]);
    expect(['/', '/settings/general', '/workspace/x'].map(d.goesTo)).toEqual([true, true, true]);
  });

  it('the settings window does none of that, and goes only to settings pages', () => {
    const d = windowDuties('settings');
    expect([d.renewsCredentials, d.announcesIdentities, d.handlesMenu]).toEqual([false, false, false]);
    expect(['/settings/dojo', '/', '/logs'].map(d.goesTo)).toEqual([true, false, false]);
  });

  it('a workspace window does none of it either, and stays on workspace pages', () => {
    const d = windowDuties('workspace-abc');
    expect([d.renewsCredentials, d.announcesIdentities, d.handlesMenu]).toEqual([false, false, false]);
    expect(['/workspace/abc/overview', '/'].map(d.goesTo)).toEqual([true, false]);
  });
});
