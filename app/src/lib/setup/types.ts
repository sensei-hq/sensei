// Setup wizard — shared types not covered by contracts.ts.
// Daemon response types live in contracts.ts. Stage definitions live in stages.ts.

export interface RoleOption {
  id: string;
  label: string;
  kanji: string;
  /** The Solar Bold Duotone class shown in icon mode (#133). */
  icon: string;
}

/** Every role a repository can carry: exactly the values of `sensei.folder_role`
 *  (a spec reads the DDL to keep them equal). A picker listing fewer showed a
 *  stored `tool` or `website` as blank (#271). */
export const ROLES: RoleOption[] = [
  { id: 'backend',   label: 'Backend',   kanji: '後', icon: 'i-solar-server-bold-duotone' },
  { id: 'frontend',  label: 'Frontend',  kanji: '前', icon: 'i-solar-monitor-bold-duotone' },
  { id: 'library',   label: 'Library',   kanji: '書', icon: 'i-solar-book-bold-duotone' },
  { id: 'tool',      label: 'Tool',      kanji: '具', icon: 'i-solar-sledgehammer-bold-duotone' },
  { id: 'docs',      label: 'Docs',      kanji: '記', icon: 'i-solar-document-text-bold-duotone' },
  { id: 'infra',     label: 'Infra',     kanji: '基', icon: 'i-solar-cloud-bold-duotone' },
  { id: 'website',   label: 'Website',   kanji: '網', icon: 'i-solar-global-bold-duotone' },
  { id: 'desktop',   label: 'Desktop',   kanji: '卓', icon: 'i-solar-laptop-bold-duotone' },
  { id: 'mobile',    label: 'Mobile',    kanji: '携', icon: 'i-solar-smartphone-bold-duotone' },
  { id: 'config',    label: 'Config',    kanji: '設', icon: 'i-solar-tuning-square-bold-duotone' },
  { id: 'packaging', label: 'Packaging', kanji: '包', icon: 'i-solar-box-bold-duotone' },
  { id: 'quality',   label: 'Quality',   kanji: '質', icon: 'i-solar-shield-check-bold-duotone' },
];
