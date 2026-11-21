import { lang, type Lang } from './i18n.svelte';

export const THEMES = [
  { id: 'hell', name: 'theme_light', style: 'modern', color: '#ffffff', swatch: '#ffffff' },
  { id: 'asche', name: 'theme_ash', style: 'modern', color: '#34363c', swatch: '#34363c' },
  { id: 'dunkel', name: 'theme_dark', style: 'modern', color: '#1b1b1f', swatch: '#1b1b1f' },
  { id: 'onyx', name: 'theme_onyx', style: 'modern', color: '#000000', swatch: '#000000' },
  { id: 'modern', name: 'theme_midnight', style: 'modern', color: '#1a1b2e', swatch: '#23254a' },
  { id: 'pergament', name: 'theme_parchment', style: 'pergament', color: '#24170f', swatch: '#ecdcb8' },
  { id: 'nordland', name: 'theme_nordland', style: 'nordland', color: '#140e09', swatch: 'linear-gradient(135deg, #3a2a1d 0 55%, #f08a2c 55%)' },
  { id: 'neon', name: 'theme_neon', style: 'neon', color: '#05060b', swatch: 'linear-gradient(135deg, #0a0c16 0 55%, #fcee0a 55% 78%, #00f0ff 78%)' },
  { id: 'aurum', name: 'theme_aurum', style: 'aurum', color: '#12100c', swatch: 'linear-gradient(135deg, #1d1a14 0 55%, #c9a24a 55%)' },
  { id: 'hain', name: 'theme_grove', style: 'hain', color: '#18211b', swatch: 'linear-gradient(135deg, #27342b 0 55%, #c9a65a 55%)' },
] as const;

export type ThemeId = (typeof THEMES)[number]['id'];

const stored = localStorage.getItem('hearth_theme');

export const theme = $state({ id: (THEMES.some((t) => t.id === stored) ? stored : 'dunkel') as ThemeId });

type Words = Record<Lang, string[]>;

const VOCAB = ['pergament', 'nordland', 'neon', 'aurum', 'hain'];

const WORDS: Record<string, Words> = {
  server: {
    de: ['Server', 'Gilde', 'Langhaus', 'Netz', 'Relais', 'Hain'],
    en: ['server', 'guild', 'longhouse', 'net', 'relay', 'grove'],
    da: ['server', 'laug', 'langhus', 'net', 'relæ', 'lund'],
  },
  servers: {
    de: ['Server', 'Gilden', 'Langhäuser', 'Netze', 'Relais', 'Haine'],
    en: ['Servers', 'Guilds', 'Longhouses', 'Nets', 'Relays', 'Groves'],
    da: ['Servere', 'Laug', 'Langhuse', 'Net', 'Relæer', 'Lunde'],
  },
  channels: {
    de: ['Kanäle', 'Hallen', 'Feuerstellen', 'Frequenzen', 'Kammern', 'Lichtungen'],
    en: ['Channels', 'Halls', 'Hearths', 'Frequencies', 'Chambers', 'Clearings'],
    da: ['Kanaler', 'Haller', 'Ildsteder', 'Frekvenser', 'Kamre', 'Lysninger'],
  },
  members: {
    de: ['Mitglieder', 'Gefährten', 'Sippe', 'Crew', 'Kohorte', 'Wanderer'],
    en: ['Members', 'Companions', 'Kin', 'Crew', 'Cohort', 'Wanderers'],
    da: ['Medlemmer', 'Fæller', 'Slægt', 'Crew', 'Kohorte', 'Vandrere'],
  },
  dms: {
    de: ['Direktnachrichten', 'Briefe', 'Raben', 'Direktlinks', 'Transmissionen', 'Flüstern'],
    en: ['Direct messages', 'Letters', 'Ravens', 'Direct links', 'Transmissions', 'Whispers'],
    da: ['Direkte beskeder', 'Breve', 'Ravne', 'Direkte links', 'Transmissioner', 'Hvisken'],
  },
  message: {
    de: ['Nachricht', 'Botschaft', 'Kunde', 'Ping', 'Transmission', 'Wort'],
    en: ['Message', 'Missive', 'Word', 'Ping', 'Transmission', 'Word'],
    da: ['Besked', 'Budskab', 'Bud', 'Ping', 'Transmission', 'Ord'],
  },
  send: {
    de: ['Senden', 'Verkünden', 'Rufen', 'Übertragen', 'Übermitteln', 'Flüstern'],
    en: ['Send', 'Proclaim', 'Call out', 'Upload', 'Transmit', 'Whisper'],
    da: ['Send', 'Forkynd', 'Råb', 'Upload', 'Overfør', 'Hvisk'],
  },
  roles: {
    de: ['Rollen', 'Ränge', 'Stände', 'Freigaben', 'Ränge', 'Pfade'],
    en: ['Roles', 'Ranks', 'Standings', 'Clearances', 'Ranks', 'Paths'],
    da: ['Roller', 'Rang', 'Stænder', 'Adgange', 'Rang', 'Stier'],
  },
  invite: {
    de: ['Einladung', 'Geleitbrief', 'Ruf', 'Zugangscode', 'Siegel', 'Wegzeichen'],
    en: ['invite', 'letter of passage', 'summons', 'access code', 'seal', 'waymark'],
    da: ['invitation', 'lejdebrev', 'kald', 'adgangskode', 'segl', 'vejmærke'],
  },
  invites: {
    de: ['Einladungen', 'Geleitbriefe', 'Rufe', 'Zugangscodes', 'Siegel', 'Wegzeichen'],
    en: ['Invites', 'Letters of passage', 'Summons', 'Access codes', 'Seals', 'Waymarks'],
    da: ['Invitationer', 'Lejdebreve', 'Kald', 'Adgangskoder', 'Segl', 'Vejmærker'],
  },
  online: {
    de: ['Online', 'Anwesend', 'Am Feuer', 'Eingeloggt', 'Wach', 'Wach'],
    en: ['Online', 'Present', 'By the fire', 'Jacked in', 'Awake', 'Awake'],
    da: ['Online', 'Til stede', 'Ved ilden', 'Logget på', 'Vågen', 'Vågen'],
  },
  offline: {
    de: ['Offline', 'Auf Reisen', 'Auf Fahrt', 'Ausgeloggt', 'Im Schlaf', 'Ruhend'],
    en: ['Offline', 'Travelling', 'At sea', 'Offline', 'Dormant', 'Resting'],
    da: ['Offline', 'På rejse', 'På togt', 'Logget af', 'I dvale', 'Hvilende'],
  },
  login: {
    de: ['Anmelden', 'Eintreten', 'Eintreten', 'Einstöpseln', 'Erwachen', 'Eintreten'],
    en: ['Sign in', 'Enter', 'Enter', 'Jack in', 'Awaken', 'Enter'],
    da: ['Log ind', 'Træd ind', 'Træd ind', 'Kobl på', 'Vågn', 'Træd ind'],
  },
  register: {
    de: ['Konto erstellen', 'In die Chronik eintragen', 'Der Sippe beitreten', 'Neue ID anlegen', 'Ins Archiv aufnehmen', 'Den Pfad beginnen'],
    en: ['Create account', 'Sign the chronicle', 'Join the kin', 'Create new ID', 'Enter the archive', 'Begin the path'],
    da: ['Opret konto', 'Skriv dig i krøniken', 'Bliv en del af slægten', 'Opret nyt ID', 'Optag i arkivet', 'Begynd stien'],
  },
};

export function w(key: string) {
  const list = WORDS[key]?.[lang];
  return list ? list[VOCAB.indexOf(theme.id) + 1] : key;
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
