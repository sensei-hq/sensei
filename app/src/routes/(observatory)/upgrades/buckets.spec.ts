import { describe, it, expect } from 'vitest';
import { bucketUpgrades, kindFor, KIND_META, type Upgrade } from './buckets.js';

const u = (over: Partial<Upgrade> = {}): Upgrade => ({
  id: 'u',
  projectId: 'p',
  projectName: 'sensei',
  title: 'x',
  why: '',
  urgency: 'medium',
  status: 'pending',
  actionType: 'write_skill',
  ...over,
});

describe('kindFor', () => {
  it('maps write_skill → skill', () => expect(kindFor('write_skill')).toBe('skill'));
  it('maps create_agent → agent', () => expect(kindFor('create_agent')).toBe('agent'));
  it('maps revise_rule + enrich_memory → rule', () => {
    expect(kindFor('revise_rule')).toBe('rule');
    expect(kindFor('enrich_memory')).toBe('rule');
  });
  it('maps audit_stale → lint', () => expect(kindFor('audit_stale')).toBe('lint'));
  it('routes unknown action_type → other (never drop)', () => {
    expect(kindFor('freshly_invented_action')).toBe('other');
  });
});

describe('bucketUpgrades', () => {
  it('routes each upgrade to its kind bucket', () => {
    const b = bucketUpgrades([
      u({ id: '1', actionType: 'write_skill' }),
      u({ id: '2', actionType: 'create_agent' }),
      u({ id: '3', actionType: 'audit_stale' }),
    ]);
    expect(b.skill.map(x => x.id)).toEqual(['1']);
    expect(b.agent.map(x => x.id)).toEqual(['2']);
    expect(b.lint .map(x => x.id)).toEqual(['3']);
  });

  it('within each bucket, orders by urgency desc then project name asc', () => {
    const b = bucketUpgrades([
      u({ id: '1', urgency: 'low',    projectName: 'apple' }),
      u({ id: '2', urgency: 'high',   projectName: 'zulu'  }),
      u({ id: '3', urgency: 'medium', projectName: 'mike'  }),
      u({ id: '4', urgency: 'high',   projectName: 'alpha' }),
    ]);
    expect(b.skill.map(x => x.id)).toEqual(['4', '2', '3', '1']);
  });

  it('meta table covers every bucket key so the UI never renders undefined', () => {
    for (const k of ['skill', 'agent', 'rule', 'lint', 'other'] as const) {
      expect(KIND_META[k]).toBeDefined();
      expect(KIND_META[k].title.length).toBeGreaterThan(0);
    }
  });
});
