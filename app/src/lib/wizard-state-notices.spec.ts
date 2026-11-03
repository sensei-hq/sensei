import { describe, it, expect } from 'vitest';
import { routerSaveNotice } from './wizard-state.svelte.js';

describe('routerSaveNotice', () => {
  it('a key that did not save: which router, then why', () => {
    expect(routerSaveNotice({ id: 'openrouter', saveState: 'failed', saveError: 'Keychain refused' })).toEqual({
      tone: 'fault', title: "Couldn't save the openrouter key", detail: 'Keychain refused',
    });
  });

  it('a failure with no reason still says something', () => {
    expect(routerSaveNotice({ id: 'x', saveState: 'failed', saveError: '' })!.detail).toBe('The daemon did not say why.');
  });

  it('none unless it failed', () => {
    for (const saveState of ['idle', 'saving', 'done']) expect(routerSaveNotice({ id: 'x', saveState, saveError: 'old' })).toBeNull();
  });
});
