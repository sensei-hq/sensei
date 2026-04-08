export const prerender = true;
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = () => {
  return json([
    {
      id: 'idea-001',
      kind: 'idea',
      name: 'sensei-mobile',
      description: 'Mobile companion app — review sessions, approve cards, get notified of blockers on the go',
      maturity: 1,
      status: 'exploring',
      cardCount: 4,
      updatedAt: '5d ago',
      tags: ['mobile', 'react-native', 'companion'],
      cards: [
        { id: 'ic1', kind: 'requirement', tag: 'REQ', title: 'Push notifications for session completion and blockers', status: 'open' },
        { id: 'ic2', kind: 'question',    tag: '?',   title: 'React Native or native Swift/Kotlin?', status: 'open' },
        { id: 'ic3', kind: 'note',        tag: 'NOTE', title: 'Can share card data model with desktop via sensei.db sync', status: 'open' },
        { id: 'ic4', kind: 'decision',    tag: 'DECISION', title: 'Read-only in v1 — no editing, just review and approval', status: 'open' },
      ],
    },
    {
      id: 'idea-002',
      kind: 'idea',
      name: 'skill-registry',
      description: 'Public registry for sharing and discovering sensei skills across the community',
      maturity: 0,
      status: 'seed',
      cardCount: 2,
      updatedAt: '1w ago',
      tags: ['community', 'skills', 'registry'],
      cards: [
        { id: 'sr1', kind: 'question', tag: '?',   title: 'Host on npm or a custom registry?', status: 'open' },
        { id: 'sr2', kind: 'note',     tag: 'NOTE', title: 'Skills need a standard manifest format for registry listing', status: 'open' },
      ],
    },
    {
      id: 'idea-003',
      kind: 'idea',
      name: 'multi-agent-sessions',
      description: 'Coordinate multiple Claude Code agents on different parts of a repo simultaneously',
      maturity: 0,
      status: 'seed',
      cardCount: 1,
      updatedAt: '2w ago',
      tags: ['multi-agent', 'experimental'],
      cards: [
        { id: 'ma1', kind: 'question', tag: '?', title: 'How do we prevent conflicting file edits across agents?', status: 'open' },
      ],
    },
  ]);
};
