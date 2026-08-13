// Pure view logic for the project window sidebar. Kept out of the component so
// the section model, the active-route test, and the health readout formatting
// are unit-testable without mounting (the component itself only reads `page`
// and renders these).
import { ftrPctLabel } from '$lib/ftr.js';

export interface SidebarSection {
    id: string;
    kanji: string;
    label: string;
}

/** The project window's sections, in nav order. */
export const SECTIONS: SidebarSection[] = [
    { id: 'intake', kanji: '門', label: 'Intake' },
    { id: 'overview', kanji: '見', label: 'Overview' },
    { id: 'metrics', kanji: '計', label: 'Metrics' },
    { id: 'sessions', kanji: '録', label: 'Sessions' },
    { id: 'memories', kanji: '憶', label: 'Memories' },
    { id: 'traceability', kanji: '跡', label: 'Traceability' },
    { id: 'atlas', kanji: '図', label: 'Atlas' },
    { id: 'libraries', kanji: '蔵', label: 'Libraries' },
    { id: 'instruments', kanji: '器', label: 'Instruments' },
    { id: 'patterns', kanji: '型', label: 'Patterns' },
    { id: 'impact', kanji: '響', label: 'Impact' },
    { id: 'about', kanji: '情', label: 'About' },
];

/** True when the current path is within the given project section. */
export function isSectionActive(pathname: string, projectId: string, sectionId: string): boolean {
    return pathname.startsWith(`/project/${projectId}/${sectionId}`);
}

/** The running build's app version (Vite `define` from package.json), surfaced
 *  in the sidebar footer so it's obvious which version is in front of you — and
 *  whether it matches the daemon after a deploy. */
export const APP_VERSION: string = __SENSEI_APP_VERSION__;

export interface HealthRow {
    label: string;
    value: string;
}

/**
 * The small "Health" readout at the foot of the sidebar. FTR renders the no-data
 * em dash when absent (never a fabricated 0%), and sessions likewise falls back
 * to the em dash rather than inventing a 0.
 */
export function healthRows(ftr14d: number | null, sessions7d: number | null): HealthRow[] {
    return [
        { label: 'FTR · 14d', value: ftrPctLabel(ftr14d) },
        { label: 'Sessions · 7d', value: sessions7d == null ? '—' : String(sessions7d) },
    ];
}
