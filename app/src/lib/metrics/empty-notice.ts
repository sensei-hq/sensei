// What the workspace metrics screen says when it has no metrics (#144).
//
// sensei computes metrics per repository — from the repository's git history
// and the sessions that ran in it — so a workspace with no repository on this
// machine will never have any. Measured on this install: 130 of 133 workspaces
// have none, and each gets zero metric rows. "Not computed yet" there promised
// a value that is not coming; this says why, once, and what would change it.

export type EmptyMetricsNotice = { kind: 'no-repository' | 'pending'; text: string };

export function emptyMetricsNotice(reposCount: number | undefined): EmptyMetricsNotice {
  if (reposCount === 0) {
    return {
      kind: 'no-repository',
      text:
        'This workspace has no repository on this machine. sensei measures each repository from its git history ' +
        'and the sessions that run in it, so no metrics will appear here until the workspace includes a git repository.',
    };
  }
  return { kind: 'pending', text: 'No metrics have been computed for this workspace yet.' };
}
