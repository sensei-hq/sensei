// Setup wizard — shared types not covered by contracts.ts.
// Daemon response types live in contracts.ts. Stage definitions live in stages.ts.

export interface RoleOption {
  id: string;
  label: string;
  kanji: string;
}

/** Every role a repository can carry: exactly the values of `sensei.folder_role`
 *  (a spec reads the DDL to keep them equal). A picker listing fewer showed a
 *  stored `tool` or `website` as blank (#271). */
export const ROLES: RoleOption[] = [
  { id: 'backend',   label: 'Backend',   kanji: '後' },
  { id: 'frontend',  label: 'Frontend',  kanji: '前' },
  { id: 'library',   label: 'Library',   kanji: '書' },
  { id: 'tool',      label: 'Tool',      kanji: '具' },
  { id: 'docs',      label: 'Docs',      kanji: '記' },
  { id: 'infra',     label: 'Infra',     kanji: '基' },
  { id: 'website',   label: 'Website',   kanji: '網' },
  { id: 'desktop',   label: 'Desktop',   kanji: '卓' },
  { id: 'mobile',    label: 'Mobile',    kanji: '携' },
  { id: 'config',    label: 'Config',    kanji: '設' },
  { id: 'packaging', label: 'Packaging', kanji: '包' },
  { id: 'quality',   label: 'Quality',   kanji: '質' },
];
