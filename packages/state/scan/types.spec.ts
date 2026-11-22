import { describe, it, expect } from 'vitest';
import { parseScanStreamEvent } from './types.js';

// The scan socket speaks the uniform envelope: validated like the relay
// stream (action + entity required), envelope extras carried through.
describe('parseScanStreamEvent', () => {
  const frame = {
    v: 1,
    id: 41,
    ts: '2026-11-21T10:00:00.000Z',
    entity: 'folder',
    action: 'update',
    key: '/code/app',
    data: { id: 'f1' },
  };

  it('accepts a full frame', () => {
    expect(parseScanStreamEvent(JSON.stringify(frame))).toEqual(frame);
  });

  it('rejects non-objects and unknown actions', () => {
    expect(() => parseScanStreamEvent('42')).toThrow(/not an object/);
    expect(() => parseScanStreamEvent(JSON.stringify({ ...frame, action: 'nuke' }))).toThrow(
      /no action/,
    );
  });

  it('rejects frames naming no entity or no data object', () => {
    expect(() => parseScanStreamEvent(JSON.stringify({ ...frame, entity: 7 }))).toThrow(
      /no entity/,
    );
    expect(() => parseScanStreamEvent(JSON.stringify({ ...frame, data: null }))).toThrow(
      /no entity/,
    );
  });
});
