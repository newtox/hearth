import { fromEnglish, lang, locale, t } from './i18n.svelte';
import type { Badge } from './store.svelte';

export const TRACKS: Record<string, { name: string; hint: string; years?: boolean }> = {
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
  const local = lang === 'en' ? null : b.translations?.[lang];
  return { name: local?.name || fromEnglish(b.name), description: local?.description || fromEnglish(b.description) };
}
