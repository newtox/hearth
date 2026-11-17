import { DA } from './lang/da';
import { DE } from './lang/de';
import { EN } from './lang/en';

export type Lang = 'de' | 'en' | 'da';

const stored = localStorage.getItem('hearth_lang');

const browser = navigator.language.toLowerCase();

export const lang: Lang = stored === 'de' || stored === 'en' || stored === 'da' ? stored : browser.startsWith('de') ? 'de' : browser.startsWith('da') ? 'da' : 'en';

export const LANGS: { id: Lang; name: string }[] = [
  { id: 'de', name: 'Deutsch' },
  { id: 'en', name: 'English' },
  { id: 'da', name: 'Dansk' },
];

export function setLang(id: Lang) {
  localStorage.setItem('hearth_lang', id);
  location.reload();
}

export function locale() {
  return lang === 'de' ? 'de-DE' : lang === 'da' ? 'da-DK' : 'en-GB';
}

document.documentElement.lang = lang;

const TEXTS: Record<Lang, Record<string, string>> = { de: DE, en: EN, da: DA };

export function t(key: string, vars?: Record<string, string | number>) {
  let s = TEXTS[lang][key] ?? EN[key] ?? key;
  if (vars) for (const k in vars) s = s.replaceAll(`{${k}}`, String(vars[k]));
  return s;
}

const BY_ENGLISH: Record<string, string> = Object.fromEntries(Object.entries(EN).map(([key, text]) => [text, key]));

export function fromEnglish(text: string) {
  const key = BY_ENGLISH[text];
  return key ? t(key) : text;
}

const LIMITS: Record<Lang, { range: string; most: string; image: string }> = {
  de: { range: 'Zeichen', most: 'höchstens {n} Zeichen', image: 'Bild ist zu groß (max. {n} MB)' },
  en: { range: 'characters', most: 'at most {n} characters', image: 'The image is too large (max. {n} MB)' },
  da: { range: 'tegn', most: 'højst {n} tegn', image: 'Billedet er for stort (maks. {n} MB)' },
};

export function serverError(message: string) {
  if (BY_ENGLISH[message]) return t(BY_ENGLISH[message]);
  const l = LIMITS[lang];
  let m = message.match(/^(.+): 1–(\d+) characters$/);
  if (m) return `${fromEnglish(m[1])}: 1–${m[2]} ${l.range}`;
  m = message.match(/^(.+): at most (\d+) characters$/);
  if (m) return `${fromEnglish(m[1])}: ${l.most.replace('{n}', m[2])}`;
  m = message.match(/^The image is too large \(max\. (\d+) MB\)$/);
  if (m) return l.image.replace('{n}', m[1]);
  return message;
}

export function auditDetail(detail: string) {
  if (lang === 'de') return detail.replace(/^(\d+) minutes$/, '$1 Minuten');
  if (lang === 'da') return detail.replace(/^(\d+) minutes$/, '$1 minutter');
  return detail;
}
