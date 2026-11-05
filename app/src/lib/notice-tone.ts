// The Notice tones as a type, derived from the one table that defines them
// (packages/ui/notice.js NOTICE_TONES) — so no screen redeclares the union.
import type { NOTICE_TONES } from '@sensei/ui';

export type NoticeTone = keyof typeof NOTICE_TONES;
