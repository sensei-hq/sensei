/** Kanji → the Solar Bold Duotone class that stands for it (#133). */
export const KANJI_ICONS: Record<string, string>;
/** The icon for a kanji that is data — a workspace's, an organisation's, a dōjō's own. */
export const KIND_ICONS: { workspace: string; organisation: string; dojo: string };
/** The vocabulary icon for a kanji; none for one the vocabulary lacks. */
export function iconForKanji(kanji: string): string | undefined;
