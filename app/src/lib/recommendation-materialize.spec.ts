import { describe, it, expect } from 'vitest';
import { materializeOptions } from './recommendation-materialize.js';

// Review r1 M4: one source for the materialize options — the preview. Overview
// (`decideRecommendation`) and the metric detail's action items both read it.

describe('materializeOptions', () => {
    it('a rule echoes the scope and tier the preview proposed', () => {
        expect(materializeOptions({
            materializable: true, kind: 'rule', gov_scope: 'workspace', enforcement: 'recommended',
        })).toEqual({ gov_scope: 'workspace', enforcement: 'recommended' });
        expect(materializeOptions({
            materializable: true, kind: 'rule', gov_scope: 'repository', enforcement: 'advisory',
        })).toEqual({ gov_scope: 'repository', enforcement: 'advisory' });
    });

    it('a file the person must consent to sends nothing extra', () => {
        expect(materializeOptions({
            materializable: true, kind: 'skill', path: '.claude/skills/x/SKILL.md', consent_required: true,
            // A file preview never proposes a rule scope; if one came, it is still not sent.
            gov_scope: 'workspace', enforcement: 'recommended',
        })).toEqual({});
    });

    it('a scope or tier the preview left out is left out, so the daemon’s default applies', () => {
        expect(materializeOptions({ materializable: true, kind: 'rule' })).toEqual({});
        expect(materializeOptions({ materializable: true, kind: 'rule', enforcement: 'required' }))
            .toEqual({ enforcement: 'required' });
    });
});
