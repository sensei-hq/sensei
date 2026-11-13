// The task view as the r1 mockup draws it (Relay.dc.html, view "task", the merged
// "Diagrams header on ScreenShell"): the bench renders these beside the artboard.
import type { RelayHandoffView, RelayRunDetailsView, RelayTaskHeaderView } from '@sensei/state';

export const mockupTaskHeader = (): RelayTaskHeaderView => ({
  back: { label: 'Now', href: '/relay' },
  title: 'Diagrams header on ScreenShell',
  state: { label: 'merged', tone: 'success' },
  ticket: { provider: 'Jira', key: 'SEN-52', url: 'https://example.atlassian.net/browse/SEN-52' },
  facts: [
    { k: 'Lane', v: 'Diagrams · L1 · cobalt-lyra' },
    { k: 'Assistant', v: 'Claude Code 2.1 · claude-opus-5-5' },
    { k: 'Time', v: '06:00 → 10:02 · 4 h 2 min' },
    { k: 'Rounds', v: 'tests 1 · impl 2' },
  ],
});

export const mockupRunDetails = (): RelayRunDetailsView => ({
  line: '3.1 M tokens in · 2 h 22 min working · 1 subagent',
  rows: [
    { k: 'Repository', v: 'sensei-hq/sensei' },
    { k: 'Branch', v: 'lane/L1' },
    { k: 'Commits', v: '3f2a91c, 8be04d7' },
    { k: 'Model', v: 'claude-opus-5-5' },
    { k: 'Tokens in', v: '3,100,000' },
    { k: 'Tokens out', v: '112,000' },
    { k: 'Tokens cached', v: null },
    { k: 'Peak context', v: '118,000' },
    { k: 'Compactions', v: '0' },
    { k: 'Peak memory', v: null },
    { k: 'Subagents', v: '1' },
    { k: 'Working', v: '2 h 22 min' },
    { k: 'Waiting for review', v: '30 min' },
    { k: 'Files changed', v: '6 · 1 outside owned paths' },
    { k: 'Gates run', v: '6' },
  ],
  note: 'Totals over the task’s turns · 2 of 4 reported',
});

export const mockupHandoff = (): RelayHandoffView => ({
  label: 'Handed off · 06:00 by the coordinator',
  brief:
    'Move the diagrams header onto the shared ScreenShell (D87): mark, title, context and toolbar in the title bar, the info band in the first-session look, panels remembered per view id. Nothing the header does today may be lost — every existing spec stays green unedited — and the report carries a before → after parity list.',
  docs: ['coord/mockups/screen-shell/CONTRACT.md', 'coord/briefs/L1-diagrams-shell.md'],
});
