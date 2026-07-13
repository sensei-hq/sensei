import { describe, it, expect } from 'vitest';
import { AtlasView } from './atlas-view.svelte.js';

describe('AtlasView', () => {
  it('defaults to the community overview with nothing focused', () => {
    const v = new AtlasView();
    expect(v.level).toBe('communities');
    expect(v.selectedId).toBeNull();
  });

  it('honours the initial level', () => {
    expect(new AtlasView('symbols').level).toBe('symbols');
  });

  it('toggles a node on repeat select', () => {
    const v = new AtlasView();
    v.select('n1');
    expect(v.selectedId).toBe('n1');
    v.select('n1');
    expect(v.selectedId).toBeNull();
  });

  it('switches focus to a different node', () => {
    const v = new AtlasView();
    v.select('n1');
    v.select('n2');
    expect(v.selectedId).toBe('n2');
  });

  it('drops the selection when the level changes', () => {
    const v = new AtlasView('symbols');
    v.select('n1');
    v.setLevel('communities');
    expect(v.level).toBe('communities');
    expect(v.selectedId).toBeNull();
  });

  it('keeps the selection when setLevel is a no-op', () => {
    const v = new AtlasView('symbols');
    v.select('n1');
    v.setLevel('symbols');
    expect(v.selectedId).toBe('n1');
  });

  it('clears the selection', () => {
    const v = new AtlasView();
    v.select('n1');
    v.clear();
    expect(v.selectedId).toBeNull();
  });
});
