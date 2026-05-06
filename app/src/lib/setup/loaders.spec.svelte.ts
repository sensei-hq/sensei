import { describe, it, expect } from 'vitest';
import { extractCompletion, extractPreferences } from './loaders.js';

describe('extractCompletion', () => {
  it('returns pending for missing keys', () => {
    const result = extractCompletion({});
    expect(result.welcome).toBe('pending');
    expect(result.roots).toBe('pending');
    expect(result.done).toBe('pending');
  });

  it('returns done for set keys', () => {
    const result = extractCompletion({ 'setup.welcome': 'done', 'setup.roots': 'done' });
    expect(result.welcome).toBe('done');
    expect(result.roots).toBe('done');
    expect(result.scan).toBe('pending');
  });

  it('returns pending for non-done values', () => {
    const result = extractCompletion({ 'setup.welcome': 'something-else' });
    expect(result.welcome).toBe('pending');
  });

  it('covers all 11 stages', () => {
    const result = extractCompletion({});
    const keys = Object.keys(result);
    expect(keys).toContain('welcome');
    expect(keys).toContain('preferences');
    expect(keys).toContain('assistants');
    expect(keys).toContain('roots');
    expect(keys).toContain('scan');
    expect(keys).toContain('projects');
    expect(keys).toContain('libraries');
    expect(keys).toContain('instruments');
    expect(keys).toContain('inference');
    expect(keys).toContain('assignments');
    expect(keys).toContain('done');
    expect(keys).toHaveLength(11);
  });
});

describe('extractPreferences', () => {
  it('returns defaults when no config key', () => {
    const result = extractPreferences({});
    expect(result.displayName).toBe('');
    expect(result.contributeLearnings).toBe(true);
    expect(result.anonymizedTelemetry).toBe(false);
  });

  it('returns stored object when config key exists', () => {
    const result = extractPreferences({
      'setup.preferences': {
        displayName: 'Jerry',
        contributeLearnings: false,
        anonymizedTelemetry: true,
      },
    });
    expect(result.displayName).toBe('Jerry');
    expect(result.contributeLearnings).toBe(false);
    expect(result.anonymizedTelemetry).toBe(true);
  });

  it('merges partial stored object with defaults', () => {
    const result = extractPreferences({
      'setup.preferences': { displayName: 'Keiko' },
    });
    expect(result.displayName).toBe('Keiko');
    expect(result.contributeLearnings).toBe(true);
    expect(result.shareSchedule).toBe('weekly-saturday');
  });

  it('ignores non-object values', () => {
    const result = extractPreferences({ 'setup.preferences': 'not-an-object' });
    expect(result.displayName).toBe('');
  });
});
