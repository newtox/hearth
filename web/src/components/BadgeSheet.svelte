<script lang="ts">
  import { backLayer } from '../lib/back.svelte';
  import { TRACKS, amount, badgeLabel } from '../lib/activity';
  import { locale, t } from '../lib/i18n.svelte';
  import type { Badge } from '../lib/store.svelte';
  import BadgeIcon from './BadgeIcon.svelte';
  import Icon from './Icon.svelte';

  let { badge, name, onprogress, onsince, onclose }: { badge: Badge; name: string; onprogress?: () => void; onsince?: (day: string) => Promise<void>; onclose: () => void } = $props();

  let editing = $state(false);
  let day = $state('');
  const iso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

  function editSince() {
    day = iso(new Date(badge.granted_at ?? Date.now()));
    editing = true;
  }

  async function saveSince() {
    if (!day || !onsince) return;
    await onsince(day);
    editing = false;
  }

  const BUILT_IN: Record<string, string> = {
    '00000000-0000-7000-8000-000000000002': 'badge_sheet_part_team_builds_runs_instance',
    '00000000-0000-7000-8000-000000000003': 'badge_sheet_supports_running_instance',
  };

  const label = $derived(badgeLabel(badge));
  const title = $derived(badge.track ? t(TRACKS[badge.track].name) : label.name);
  const text = $derived(
    badge.track
      ? t(TRACKS[badge.track].hint)
      : BUILT_IN[badge.id]
        ? t(BUILT_IN[badge.id])
        : label.description || t('badge_sheet_given_admins_instance'),
  );
  const date = new Intl.DateTimeFormat(locale(), { dateStyle: 'long' });

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={key} />

<div class="veil" use:backLayer={onclose} onclick={(e) => e.target === e.currentTarget && onclose()} role="presentation">
  <div class="sheet" role="dialog" aria-modal="true" aria-label={title}>
    <span class="grip" aria-hidden="true"></span>
    <button class="x" onclick={onclose} aria-label={t('common_close')}><Icon name="close" /></button>
    <div class="art" style:--c={badge.color}><BadgeIcon {badge} size={84} /></div>
    <h2>{title}</h2>
    <div class="meta">
      {#if badge.track}
        <span>{t('badge_sheet_earned_badge')}</span>
        <span class="level" style:--c={badge.color}>{t('common_level_tier_10', { tier: (badge.tier ?? 0) + 1 })}</span>
      {:else}
        {#if editing}
          <form class="since" onsubmit={(e) => (e.preventDefault(), saveSince())}>
            <label for="badge-since">{t('badge_sheet_since_label')}</label>
            <input id="badge-since" class="input" type="date" bind:value={day} min="2000-01-01" max={iso(new Date())} required />
            <button class="btn small primary">{t('common_save')}</button>
            <button type="button" class="btn small" onclick={() => (editing = false)}>{t('common_cancel')}</button>
          </form>
        {:else}
          <span>{badge.granted_at ? t('badge_sheet_instance_badge_since', { date: date.format(new Date(badge.granted_at)) }) : t('badge_sheet_instance_badge')}</span>
          {#if onsince && badge.granted_at}<button class="edit" onclick={editSince} aria-label={t('badge_sheet_change_since')} title={t('badge_sheet_change_since')}><Icon name="edit" size={14} /></button>{/if}
        {/if}
      {/if}
    </div>
    <div class="box">
      <small>{badge.track ? t('badge_sheet_how_get') : t('common_description')}</small>
      <p>{text}</p>
      {#if badge.track}
        <p class="muted">{t('badge_sheet_name_has_amount_so_far', { name, amount: amount(badge.track, badge.hours ?? 0) })}</p>
      {/if}
    </div>
    {#if badge.track && onprogress}
      <button class="btn" onclick={onprogress}>{t('badge_sheet_view_progress')}</button>
    {/if}
  </div>
</div>

<style>
  .veil {
    animation: fade-in 0.14s ease-out;
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 0.55);
  }
  .sheet {
    animation: pop-in 0.16s cubic-bezier(0.2, 0.9, 0.3, 1);
    position: relative;
    width: min(420px, 100%);
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 28px 22px 22px;
    border-radius: var(--radius);
    background: var(--panel);
    border: var(--panel-border);
    box-shadow: var(--panel-shadow);
    text-align: center;
  }
  .grip {
    display: none;
  }
  .x {
    position: absolute;
    top: 10px;
    right: 10px;
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .x:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .art {
    display: grid;
    place-items: center;
    width: 120px;
    height: 120px;
    border-radius: 28px;
    background: radial-gradient(circle, color-mix(in srgb, var(--c) 24%, transparent), transparent 70%);
  }
  h2 {
    margin: 0;
    font-size: 1.5em;
    letter-spacing: 0.01em;
  }
  .meta {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .edit {
    display: grid;
    place-items: center;
    padding: 4px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .edit:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .since {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: 8px;
  }
  .since .input {
    width: auto;
  }
  .level::first-letter {
    text-transform: uppercase;
  }
  .level {
    display: inline-block;
    padding: 1px 10px;
    border-radius: 99px;
    border: 1px solid var(--c);
    color: var(--c);
    font-weight: 700;
    font-size: 0.88em;
  }
  .box {
    width: 100%;
    margin-top: 6px;
    padding: 14px 16px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    text-align: left;
  }
  .box small {
    color: var(--muted);
    font-weight: 600;
  }
  .box p {
    margin: 4px 0 0;
  }
  .btn {
    width: 100%;
  }
  @media (max-width: 600px) {
    .veil {
      place-items: end stretch;
      padding: 0;
    }
    .sheet {
      width: 100%;
      border-radius: 22px 22px 0 0;
      padding-bottom: max(22px, env(safe-area-inset-bottom));
    }
    .grip {
      display: block;
      width: 44px;
      height: 5px;
      border-radius: 3px;
      background: var(--line);
      margin-top: -14px;
    }
  }
</style>
