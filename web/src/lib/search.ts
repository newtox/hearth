import { lang } from './i18n.svelte';
import type { Channel, Guild, User } from './store.svelte';

export type FilterKey = 'from' | 'in' | 'has' | 'mentions' | 'before' | 'after' | 'during' | 'pinned';
export type HasKind = 'link' | 'file' | 'image' | 'video' | 'sound' | 'sticker' | 'poll';

const KEYS: Record<FilterKey, [string, string, string]> = {
  from: ['from', 'von', 'fra'],
  in: ['in', 'in', 'i'],
  has: ['has', 'hat', 'har'],
  mentions: ['mentions', 'erwähnt', 'nævnt'],
  before: ['before', 'vor', 'før'],
  after: ['after', 'nach', 'efter'],
  during: ['during', 'am', 'under'],
  pinned: ['pinned', 'angeheftet', 'fastgjort'],
};

const HAS: Record<HasKind, [string, string, string]> = {
  link: ['link', 'link', 'link'],
  file: ['file', 'datei', 'fil'],
  image: ['image', 'bild', 'billede'],
  video: ['video', 'video', 'video'],
  sound: ['sound', 'ton', 'lyd'],
  sticker: ['sticker', 'sticker', 'klistermærke'],
  poll: ['poll', 'umfrage', 'afstemning'],
};

const YES = ['true', 'yes', 'ja'];
const slot = lang === 'en' ? 0 : lang === 'de' ? 1 : 2;

export const HAS_KINDS = Object.keys(HAS) as HasKind[];

export function keyword(key: FilterKey) {
  return KEYS[key][slot];
}

export function hasWord(kind: HasKind) {
  return HAS[kind][slot];
}

export function yesWord() {
  return lang === 'en' ? 'true' : 'ja';
}

function findKey(word: string): FilterKey | null {
  const w = word.toLowerCase();
  return (Object.keys(KEYS) as FilterKey[]).find((k) => KEYS[k].includes(w)) ?? null;
}

function findHas(word: string): HasKind | null {
  const w = word.toLowerCase();
  return HAS_KINDS.find((k) => HAS[k].includes(w)) ?? null;
}

const TOKEN = /(?<![^\s])([\p{L}]+):("[^"]*"|[^\s"]*)/gu;

export function editing(query: string): { key: FilterKey; value: string; start: number } | null {
  const m = query.match(/(?:^|\s)([\p{L}]+):("[^"]*|[^\s"]*)$/u);
  if (!m) return null;
  const key = findKey(m[1]);
  return key ? { key, value: m[2].replace(/^"/, ''), start: query.length - m[0].trimStart().length } : null;
}

function day(value: string) {
  const m = value.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!m) return null;
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  return Number.isNaN(d.getTime()) ? null : d;
}

function nextDay(d: Date) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
}

export function toParams(query: string, g: Guild | null, people: User[]) {
  const params = new URLSearchParams();
  const has = new Set<HasKind>();
  const text = query
    .replace(TOKEN, (whole, rawKey: string, quoted: string) => {
      const key = findKey(rawKey);
      const value = quoted.replace(/^"|"$/g, '').trim();
      if (!key) return whole;
      if (!value) return ' ';
      if (key === 'from' || key === 'mentions') {
        const name = value.replace(/^@/, '').toLowerCase();
        const user = people.find((u) => u.username.toLowerCase() === name.toLowerCase() || u.display_name.toLowerCase() === name.toLowerCase());
        params.set(key === 'from' ? 'author' : 'mentions', user?.username ?? name);
        return ' ';
      }
      if (key === 'in') {
        const ch = g?.channels.find((c: Channel) => c.kind === 'text' && c.name.toLowerCase() === value.replace(/^#/, '').toLowerCase());
        if (!ch) return whole;
        params.set('channel_id', ch.id);
        return ' ';
      }
      if (key === 'has') {
        const kind = findHas(value);
        if (!kind) return whole;
        has.add(kind);
        return ' ';
      }
      if (key === 'pinned') {
        if (YES.includes(value.toLowerCase())) params.set('pinned', 'true');
        return ' ';
      }
      const d = day(value);
      if (!d) return whole;
      if (key === 'before') params.set('before', d.toISOString());
      if (key === 'after') params.set('after', nextDay(d).toISOString());
      if (key === 'during') {
        params.set('after', d.toISOString());
        params.set('before', nextDay(d).toISOString());
      }
      return ' ';
    })
    .replace(/\s+/g, ' ')
    .trim();
  if (has.size) params.set('has', [...has].join(','));
  if (text) params.set('q', text);
  return params;
}
