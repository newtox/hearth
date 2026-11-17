import { lang } from './i18n.svelte';

const REPO = (import.meta.env.VITE_REPO_URL as string | undefined) ?? 'https://github.com/newtox/hearth';

const PAGES = {
  installation: ['installation', 'installation'],
  apps: ['apps', 'apps'],
  configuration: ['configuration', 'einstellungen'],
  connections: ['connections', 'verbindungen'],
  permissions: ['permissions', 'rechte'],
  guide: ['guide', 'anleitung'],
} as const;

export function docs(page: keyof typeof PAGES) {
  const [en, de] = PAGES[page];
  return lang === 'de' ? `${REPO}/blob/main/docs/de/${de}.md` : `${REPO}/blob/main/docs/en/${en}.md`;
}

export const releases = `${REPO}/releases/latest`;
