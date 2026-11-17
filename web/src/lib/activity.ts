import { api, isApp } from './api';
import { fromEnglish, locale, t } from './i18n.svelte';
import type { Activity, Badge } from './store.svelte';

export function activityText(a: Activity) {
  if (a.kind === 'playing') return t('activity_playing_name', { name: a.name });
  if (a.kind === 'listening') return t('activity_listening_name', { name: a.name });
  if (a.kind === 'watching') return t('activity_watching_name', { name: a.name });
  return a.name;
}

export function elapsed(since: string) {
  const minutes = Math.max(0, Math.floor((Date.now() - Date.parse(since)) / 60000));
  if (minutes < 1) return t('activity_just_now');
  if (minutes < 60) return t('activity_n_min', { n: minutes });
  return t('activity_n_h', { n: Math.floor(minutes / 60) });
}

export const TRACKS: Record<string, { name: string; hint: string; years?: boolean }> = {
  play: { name: 'activity_playtime', hint: 'activity_counts_while_game_running_as' },
  stream: { name: 'activity_stream_time', hint: 'activity_counts_while_share_screen_voice' },
  voice: { name: 'activity_voice_time', hint: 'activity_counts_while_voice_channel' },
  age: { name: 'activity_account_age', hint: 'activity_counts_from_day_registered_time', years: true },
};

export function amount(track: string, n: number) {
  if (TRACKS[track]?.years) return n === 1 ? t('activity_1_year') : t('activity_n_years', { n });
  return n === 1 ? t('common_1_hour') : t('activity_n_hours', { n: n.toLocaleString(locale()) });
}

export function badgeLabel(b: Badge) {
  if (b.track) return { name: t(TRACKS[b.track].name), description: `${amount(b.track, b.hours ?? 0)}, ${t('common_level_tier_10', { tier: (b.tier ?? 0) + 1 })}` };
  return { name: fromEnglish(b.name), description: fromEnglish(b.description) };
}

const GAMES: [string, string][] = [
  ['cs2', 'Counter-Strike 2'],
  ['marvel-win64', 'Marvel Rivals'],
  ['streetfighter6', 'Street Fighter 6'],
  ['tekken 8', 'Tekken 8'],
  ['polaris-win64', 'Tekken 8'],
  ['discovery', 'THE FINALS'],
  ['witcher3', 'The Witcher 3'],
  ['bg3', "Baldur's Gate 3"],
  ['palworld', 'Palworld'],
  ['valheim', 'Valheim'],
  ['ts3', 'The Sims 3'],
  ['minecraft', 'Minecraft'],
  ['javaw', 'Minecraft'],
  ['leagueclient', 'League of Legends'],
  ['valorant', 'VALORANT'],
  ['fortniteclient', 'Fortnite'],
  ['rocketleague', 'Rocket League'],
  ['eldenring', 'Elden Ring'],
  ['overwatch', 'Overwatch 2'],
  ['gta5', 'Grand Theft Auto V'],
  ['r5apex', 'Apex Legends'],
  ['dota2', 'Dota 2'],
  ['helldivers2', 'Helldivers 2'],
  ['cyberpunk2077', 'Cyberpunk 2077'],
];

export function customGames(): [string, string][] {
  return (localStorage.getItem('hearth_games') ?? '')
    .split('\n')
    .map((line) => line.split('=').map((x) => x.trim()))
    .filter((p) => p.length === 2 && p[0] && p[1])
    .map((p) => [p[0].toLowerCase(), p[1]]);
}

export const detection = {
  available: isApp && !/android/i.test(navigator.userAgent),
  enabled: () => localStorage.getItem('hearth_detect') !== 'off',
  set: (on: boolean) => localStorage.setItem('hearth_detect', on ? 'on' : 'off'),
};

let detected: string | null = null;
let timer: ReturnType<typeof setInterval> | undefined;

async function scan() {
  if (!detection.enabled()) return;
  let processes: string[];
  try {
    processes = await (window as any).__TAURI_INTERNALS__.invoke('running_processes');
  } catch {
    return;
  }
  const names = processes.map((p) => p.toLowerCase());
  const hit = [...customGames(), ...GAMES].find(([needle]) => names.some((n) => n.includes(needle)))?.[1] ?? null;
  if (hit === detected) return;
  detected = hit;
  if (hit) await api('POST', '/users/@me/activity', { kind: 'playing', name: hit }).catch(() => (detected = null));
  else await api('DELETE', '/users/@me/activity').catch(() => {});
}

export function startDetection() {
  if (!detection.available) return;
  detected = null;
  scan();
  timer ??= setInterval(scan, 20000);
}
