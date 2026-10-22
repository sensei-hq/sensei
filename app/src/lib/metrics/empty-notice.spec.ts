import { describe, it, expect } from 'vitest';
import { emptyMetricsNotice } from './empty-notice';

// #144: sensei computes metrics per repository — from its git history and the
// sessions in it — so a workspace with no repository on this machine will never
// have any. "Not computed yet" promised a value that is not coming.
describe('emptyMetricsNotice', () => {
  it('says why a workspace with no repository has no metrics, and what would change it', () => {
    const n = emptyMetricsNotice(0);
    expect(n.kind).toBe('no-repository');
    expect(n.text).toMatch(/no repository/i);
    expect(n.text).toMatch(/git/i);
    expect(n.text).not.toMatch(/yet/i);
  });

  it('keeps "not computed yet" for a workspace that has a repository', () => {
    expect(emptyMetricsNotice(2)).toEqual({
      kind: 'pending',
      text: 'No metrics have been computed for this workspace yet.',
    });
  });

  it('does not guess when the repository count is unknown', () => {
    expect(emptyMetricsNotice(undefined).kind).toBe('pending');
  });
});
