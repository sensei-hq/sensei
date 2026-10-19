import { describe, expect, it } from 'vitest';
import { historyOffNotice } from './history-notice.js';

// #280: with an assistant's conversation-history reading off, its token, cost and
// session metrics stop updating. Blank metrics said nothing about why; the metrics
// screen names the assistants and what stopped.
const src = (source: string, label: string, consented: boolean) => ({ source, label, consented });

describe('historyOffNotice', () => {
  it('names the assistants whose history reading is off, and what stops', () => {
    const n = historyOffNotice([
      src('claude_code', 'Claude Code', false),
      src('zed', 'Zed', true),
      src('cursor', 'Cursor', false),
    ]);
    expect(n?.assistants).toEqual(['Claude Code', 'Cursor']);
    expect(n?.text).toBe(
      'Conversation history reading is off for Claude Code and Cursor, so their token, cost and session metrics are not updating. Turn it on in Settings → Assistants.',
    );
  });

  it('says nothing when every assistant is read', () => {
    expect(historyOffNotice([src('zed', 'Zed', true)])).toBeNull();
  });

  it('says nothing it cannot know: a failed consent read is not "all off"', () => {
    expect(historyOffNotice(null)).toBeNull();
  });
});
