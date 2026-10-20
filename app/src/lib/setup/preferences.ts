import type { PreferencesData } from './contracts.js';

/** The preferences a fresh install starts from — one copy, shared by the setup
 *  wizard, its loader and Settings → General, so a missing or corrupt config
 *  value renders the same form everywhere. */
export const DEFAULT_PREFERENCES: PreferencesData = {
  displayName: '',
  contributeLearnings: true,
  reviewBeforeShare: true,
  shareSchedule: 'weekly-saturday',
  downloadCollective: 'weekly',
  correctionAggressiveness: 'balanced',
  digestCadence: 'daily',
  nudgeOnRegression: true,
  anonymizedTelemetry: false,
  showWelcome: true,
  glyphs: 'kanji',
};
