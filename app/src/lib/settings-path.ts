/** Whether a path belongs to the settings window (user, 2026-10-27: settings is
 *  a child window, as a Mac app's preferences are). The main window sends these
 *  there; the settings window sends every other path back. Pure. */
export function isSettingsPath(path: string): boolean {
  return path === '/settings' || path.startsWith('/settings/');
}
