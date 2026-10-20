/** Kanji → the Solar Bold Duotone class that stands for it (#133). */
export const KANJI_ICONS: Record<string, string>;
/** Marks that are identities, not vocabulary: they stay kanji in both modes. */
export const IDENTITY_KANJI: Set<string>;
/** The vocabulary icon for a kanji; none for an identity mark or an unknown one. */
export function iconForKanji(kanji: string): string | undefined;
