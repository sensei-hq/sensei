// The machine's user name, as the default display name — shared by the setup
// wizard and Settings → General, so both prefill the same way.
import { hasTauri } from '../bootstrap.js';
import { STORAGE_KEYS } from '../storage-keys.js';

/** Best-effort username — async so it can call Tauri's homeDir().
 *  Each external boundary (localStorage, Tauri native, location) is
 *  wrapped separately so a failure in one source falls through to the
 *  next instead of aborting the whole guess. Failures are logged so
 *  they don't go silent. */
export async function guessUserName(): Promise<string> {
  // localStorage boundary — can throw in Safari private mode or under
  // quota pressure. typeof guard handles SSR/test envs where it's undefined.
  if (typeof localStorage !== 'undefined') {
    try {
      const stored = localStorage.getItem(STORAGE_KEYS.userName);
      if (stored) return stored;
    } catch (e) {
      console.warn('guessUserName: localStorage read failed', e);
    }
  }

  // Tauri native boundary — homeDir() can throw if the path plugin isn't
  // registered, and the dynamic import can fail if the bundle is stale.
  // Don't bail out of the guess on failure — fall through to the browser
  // pathname heuristic.
  if (hasTauri()) {
    try {
      const { homeDir } = await import('@tauri-apps/api/path');
      const home = await homeDir();
      const match = home.replace(/\/$/, '').match(/\/([^/]+)$/);
      if (match) return match[1].charAt(0).toUpperCase() + match[1].slice(1);
    } catch (e) {
      console.warn('guessUserName: Tauri homeDir() failed', e);
    }
  }

  // Browser fallback — pure string ops, can't throw.
  const pathMatch = (typeof location !== 'undefined' ? location.pathname : '')
    .match(/\/Users\/([^/]+)/);
  if (pathMatch) return pathMatch[1].charAt(0).toUpperCase() + pathMatch[1].slice(1);

  return '';
}
