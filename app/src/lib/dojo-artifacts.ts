// Dōjō artifact presentation — shared, pure, no runes.
//
// The kanji glyph + label + named-token chip classes for the six Dōjō artifact
// kinds (`dojo.artifact_kind`: principle · pattern · prompt · guard · skill ·
// agent), and the "where it came from" attribution line. Both are used by every
// Dōjō surface that renders an artifact — the downstream Upgrades inbox
// (`(observatory)/upgrades`) and the upstream Share-review gate
// (`(observatory)/share-review`) — so they live here rather than being
// duplicated per screen (a second consumer promotes a screen-local primitive to
// a shared one; see app/CLAUDE.md §5 DRY).
//
// Wire-API-wins: `type` and `attribution.mode` arrive as free strings, so an
// unrecognised value degrades to a neutral chip / echoes the raw value rather
// than being dropped.

import type { DojoUpgradeAttribution } from '$lib/types.js';

/** Named-token utility classes for a chip, plus its printed label. */
export interface ChipClasses {
  bg: string;
  text: string;
  label: string;
}

/** A type pill — kanji glyph + label + named-token classes. */
export interface TypePill {
  kanji: string;
  label: string;
  bg: string;
  text: string;
}

/** Pure: artifact type -> kanji glyph + label + chip classes. skill/agent read
 *  accent (concrete, installable operators), principle reads success, pattern /
 *  prompt read info, guard reads warning; an unrecognised type degrades to muted
 *  ink so a new wire kind is still shown, never dropped. Kanji follow the
 *  dojo-protocol payload glyphs (理 principle, 紋 pattern, 問 prompt, 守 guard,
 *  技 skill, 作 agent). The lookup IS the rendering rule, so it lives with the
 *  domain rather than in the component. */
export function typePill(type: string): TypePill {
  switch (type) {
    case 'skill':     return { kanji: '技', label: 'skill',     bg: 'bg-accent-soft',  text: 'text-accent'  };
    case 'agent':     return { kanji: '作', label: 'agent',     bg: 'bg-accent-soft',  text: 'text-accent'  };
    case 'principle': return { kanji: '理', label: 'principle', bg: 'bg-success-soft', text: 'text-success' };
    case 'pattern':   return { kanji: '紋', label: 'pattern',   bg: 'bg-info-soft',    text: 'text-info'    };
    case 'prompt':    return { kanji: '問', label: 'prompt',    bg: 'bg-info-soft',    text: 'text-info'    };
    case 'guard':     return { kanji: '守', label: 'guard',     bg: 'bg-warning-soft', text: 'text-warning' };
    default:          return { kanji: '贈', label: type || 'artifact', bg: 'bg-paper-mute', text: 'text-ink-mute' };
  }
}

/** Pure: attribution -> a short "where it came from" line. Named credits the
 *  author (and org when present); anonymous / dereferenced say so; anything else
 *  echoes the raw mode so provenance is never blank. */
export function attributionSummary(attr: DojoUpgradeAttribution | null | undefined): string {
  if (!attr) return 'unattributed';
  switch (attr.mode) {
    case 'named': {
      const author = attr.author?.trim();
      const org = attr.org?.trim();
      if (author && org) return `by ${author} · ${org}`;
      if (author) return `by ${author}`;
      if (org) return `from ${org}`;
      return 'named';
    }
    case 'anonymous':    return 'anonymous';
    case 'dereferenced': return 'source withheld';
    default:             return attr.mode || 'unattributed';
  }
}
