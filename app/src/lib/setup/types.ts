// Setup wizard — shared types not covered by contracts.ts.
// Daemon response types live in contracts.ts. Stage definitions live in stages.ts.

export interface RoleOption {
  id: string;
  label: string;
  kanji: string;
}

export const ROLES: RoleOption[] = [
  { id: 'backend',  label: 'Backend',  kanji: '後' },
  { id: 'frontend', label: 'Frontend', kanji: '前' },
  { id: 'library',  label: 'Library',  kanji: '書' },
  { id: 'docs',     label: 'Docs',     kanji: '記' },
  { id: 'infra',    label: 'Infra',    kanji: '基' },
];
