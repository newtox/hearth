import { lang, type Lang } from './i18n.svelte';

export const THEMES = [
  { id: 'hell', name: 'Hell', style: 'modern', color: '#ffffff', swatch: '#ffffff' },
  { id: 'asche', name: 'Asche', style: 'modern', color: '#34363c', swatch: '#34363c' },
  { id: 'dunkel', name: 'Dunkel', style: 'modern', color: '#1b1b1f', swatch: '#1b1b1f' },
  { id: 'onyx', name: 'Onyx', style: 'modern', color: '#000000', swatch: '#000000' },
  { id: 'modern', name: 'Mitternacht', style: 'modern', color: '#1a1b2e', swatch: '#23254a' },
  { id: 'pergament', name: 'Pergament', style: 'pergament', color: '#24170f', swatch: '#ecdcb8' },
] as const;

export type ThemeId = (typeof THEMES)[number]['id'];

const stored = localStorage.getItem('hearth_theme');

export const theme = $state({ id: (THEMES.some((t) => t.id === stored) ? stored : 'dunkel') as ThemeId });

const WORDS: Record<string, Record<Lang, [string, string]>> = {
  server: { de: ['Server', 'Gilde'], en: ['server', 'guild'] },
  servers: { de: ['Server', 'Gilden'], en: ['Servers', 'Guilds'] },
  channels: { de: ['Kanäle', 'Hallen'], en: ['Channels', 'Halls'] },
  members: { de: ['Mitglieder', 'Gefährten'], en: ['Members', 'Companions'] },
  dms: { de: ['Direktnachrichten', 'Briefe'], en: ['Direct messages', 'Letters'] },
  message: { de: ['Nachricht', 'Botschaft'], en: ['Message', 'Missive'] },
  send: { de: ['Senden', 'Verkünden'], en: ['Send', 'Proclaim'] },
  roles: { de: ['Rollen', 'Ränge'], en: ['Roles', 'Ranks'] },
  invite: { de: ['Einladung', 'Geleitbrief'], en: ['invite', 'letter of passage'] },
  invites: { de: ['Einladungen', 'Geleitbriefe'], en: ['Invites', 'Letters of passage'] },
  online: { de: ['Online', 'Anwesend'], en: ['Online', 'Present'] },
  offline: { de: ['Offline', 'Auf Reisen'], en: ['Offline', 'Travelling'] },
  login: { de: ['Anmelden', 'Eintreten'], en: ['Sign in', 'Enter'] },
  register: { de: ['Konto erstellen', 'In die Chronik eintragen'], en: ['Create account', 'Sign the chronicle'] },
};

export function w(key: string) {
  const pair = WORDS[key]?.[lang];
  return pair ? pair[theme.id === 'pergament' ? 1 : 0] : key;
}

export function applyTheme(id: ThemeId) {
  const entry = THEMES.find((t) => t.id === id)!;
  theme.id = id;
  localStorage.setItem('hearth_theme', id);
  document.documentElement.dataset.theme = id;
  document.documentElement.dataset.style = entry.style;
  document.querySelector('meta[name=theme-color]')?.setAttribute('content', entry.color);
}

applyTheme(theme.id);
