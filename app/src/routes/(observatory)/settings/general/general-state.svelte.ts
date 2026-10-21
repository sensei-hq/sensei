// Settings · General — the preferences form and its save lifecycle. The read is
// the loader's (+page.ts); this holds the form, saves it through a `save`
// handed in, and says how the last save went. A form that could not be read is
// never saved: defaults written back would replace the user's settings (#288).
import type { PreferencesData } from '$lib/setup/contracts.js';
import { DEFAULT_PREFERENCES, fromPreferencesForm } from '../preferences-form.js';

/** What the loader read: the preferences, or why they could not be read. */
export type PreferencesRead = { prefs: PreferencesData | null; error: string | null; prefilled: boolean };

/** Writes the daemon config (the page passes appState.setConfigs). Throws on failure. */
export type SaveConfig = (config: ReturnType<typeof fromPreferencesForm>) => Promise<unknown>;

/** The toggles that save themselves, as one comparable value. */
const togglesOf = (p: PreferencesData) => `${p.nudgeOnRegression}|${p.anonymizedTelemetry}|${p.showWelcome}`;

export class PreferencesState {
  prefs = $state<PreferencesData>({ ...DEFAULT_PREFERENCES });
  loadError = $state<string | null>(null);
  /** The display name was guessed from the system user, not read. */
  prefilled = false;
  saveStatus = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  saveError = $state<string | null>(null);
  #savedToggles = '';
  #settle: ReturnType<typeof setTimeout> | null = null;

  constructor(read: PreferencesRead) {
    this.setRead(read);
  }

  /** Take a (re-)read. Its toggles are the saved ones, so it is not a change. */
  setRead(read: PreferencesRead): void {
    this.loadError = read.error;
    this.prefilled = read.prefilled;
    if (read.prefs) this.prefs = { ...read.prefs };
    this.#savedToggles = togglesOf(this.prefs);
  }

  async persist(save: SaveConfig): Promise<void> {
    if (this.loadError) return;
    if (this.#settle) clearTimeout(this.#settle);
    this.saveStatus = 'saving';
    this.saveError = null;
    try {
      await save(fromPreferencesForm(this.prefs));
      this.#savedToggles = togglesOf(this.prefs);
      this.saveStatus = 'saved';
      this.#settle = setTimeout(() => {
        if (this.saveStatus === 'saved') this.saveStatus = 'idle';
      }, 1500);
    } catch (e) {
      this.saveStatus = 'error';
      this.saveError = (e as Error).message;
    }
  }

  /** Toggles save as they change; text and selects save from their handlers. */
  async saveIfTogglesChanged(save: SaveConfig): Promise<void> {
    if (togglesOf(this.prefs) === this.#savedToggles) return;
    await this.persist(save);
  }

  /** An empty display name took the system user name; save it once so the rest
   *  of the app greets the same name. */
  async savePrefilled(save: SaveConfig): Promise<void> {
    if (!this.prefilled) return;
    this.prefilled = false;
    await this.persist(save);
  }
}
