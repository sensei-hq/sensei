import { describe, it, expect } from 'vitest';
import {
  mockWatchRoot, mockAssistant, mockProject, mockLibEntry,
  mockMcpEntry, mockPreferences, mockWizardLoadData,
} from './mock-contracts.js';

describe('contract mock factories', () => {
  it('mockWatchRoot has required fields', () => {
    const root = mockWatchRoot();
    expect(root.id).toBeTypeOf('string');
    expect(root.path).toBeTypeOf('string');
    expect(['scanning', 'watching', 'paused']).toContain(root.status);
    expect(Array.isArray(root.excluded)).toBe(true);
    expect(root.repos_found).toBeTypeOf('number');
  });

  it('mockAssistant has required fields', () => {
    const a = mockAssistant();
    expect(a.id).toBeTypeOf('string');
    expect(a.selected).toBeTypeOf('boolean');
    expect(Array.isArray(a.variants)).toBe(true);
    expect(a.variants[0].installed).toBeTypeOf('boolean');
  });

  it('mockProject has folders array', () => {
    const p = mockProject();
    expect(Array.isArray(p.folders)).toBe(true);
    expect(p.folders[0].kind).toBeTypeOf('string');
  });

  it('mockLibEntry has name, repos, and enabled', () => {
    const l = mockLibEntry();
    expect(l.name).toBeTypeOf('string');
    expect(Array.isArray(l.repos)).toBe(true);
    expect(l.repoCount).toBeTypeOf('number');
    expect(l.enabled).toBeTypeOf('boolean');
  });

  it('mockMcpEntry has project_count', () => {
    const m = mockMcpEntry();
    expect(m.project_count).toBeTypeOf('number');
    expect(m.selected).toBeTypeOf('boolean');
  });

  it('mockWizardLoadData has all slices', () => {
    const data = mockWizardLoadData();
    expect(data.completion).toBeDefined();
    expect(data.preferences.displayName).toBeTypeOf('string');
    expect(Array.isArray(data.assistantFamilies)).toBe(true);
    expect(Array.isArray(data.roots)).toBe(true);
    expect(Array.isArray(data.projects)).toBe(true);
    expect(Array.isArray(data.libraries.libs)).toBe(true);
    expect(Array.isArray(data.mcps)).toBe(true);
  });

  it('mockWatchRoot accepts overrides', () => {
    const root = mockWatchRoot({ status: 'paused', repos_found: 0 });
    expect(root.status).toBe('paused');
    expect(root.repos_found).toBe(0);
  });

  it('mockWizardLoadData accepts partial completion', () => {
    const data = mockWizardLoadData({
      completion: {
        welcome: 'done', preferences: 'done', assistants: 'done',
        roots: 'done', scan: 'done', projects: 'pending',
        libraries: 'pending', instruments: 'pending',
        inference: 'pending', done: 'pending',
      },
    });
    expect(data.completion.welcome).toBe('done');
    expect(data.completion.projects).toBe('pending');
  });
});
