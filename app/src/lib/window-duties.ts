import { isSettingsPath } from './settings-path.js';

/**
 * What a window does beyond its own pages. Every window runs the root layout,
 * so the app's global work — renewing credentials, the identity overlay, the
 * native menu's events — is the main window's alone; a second copy opened a
 * second browser tab for one renewal (review, 2026-10-27). Each window also
 * takes only the navigations that belong to it. Pure.
 */
export interface WindowDuties {
  renewsCredentials: boolean;
  announcesIdentities: boolean;
  handlesMenu: boolean;
  goesTo: (path: string) => boolean;
}

export function windowDuties(label: string): WindowDuties {
  if (label === 'main') {
    return { renewsCredentials: true, announcesIdentities: true, handlesMenu: true, goesTo: () => true };
  }
  const own = label === 'settings' ? isSettingsPath : (path: string) => path.startsWith('/workspace/');
  return { renewsCredentials: false, announcesIdentities: false, handlesMenu: false, goesTo: own };
}
