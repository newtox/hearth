<script lang="ts">
  import { TRACKS, amount, badgeLabel } from '../lib/activity';
  import { locale, t } from '../lib/i18n.svelte';
  import type { Badge } from '../lib/store.svelte';
  import BadgeIcon from './BadgeIcon.svelte';
  import Icon from './Icon.svelte';

  let { badge, name, onprogress, onclose }: { badge: Badge; name: string; onprogress?: () => void; onclose: () => void } = $props();

  const BUILT_IN: Record<string, string> = {
    '00000000-0000-7000-8000-000000000001': 'Hat diese Instanz gegründet. Das erste Konto bekommt dieses Abzeichen automatisch.',
    '00000000-0000-7000-8000-000000000002': 'Gehört zum Team, das diese Instanz entwickelt und betreibt.',
    '00000000-0000-7000-8000-000000000003': 'Unterstützt den Betrieb dieser Instanz.',
  };

  const label = $derived(badgeLabel(badge));
  const title = $derived(badge.track ? t(TRACKS[badge.track].name) : label.name);
  const text = $derived(
    badge.track
      ? t(TRACKS[badge.track].hint)
      : BUILT_IN[badge.id]
        ? t(BUILT_IN[badge.id])
        : label.description || t('Wird von den Admins dieser Instanz vergeben.'),
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

<div class="veil" onclick={(e) => e.target === e.currentTarget && onclose()} role="presentation">
  <div class="sheet" role="dialog" aria-modal="true" aria-label={title}>
    <span class="grip" aria-hidden="true"></span>
    <button class="x" onclick={onclose} aria-label={t('Schließen')}><Icon name="close" /></button>
    <div class="art" style:--c={badge.color}><BadgeIcon {badge} size={84} /></div>
    <h2>{title}</h2>
    <p class="meta">
      {#if badge.track}
        <span>{t('Verdientes Abzeichen')}</span>
        <span class="level" style:--c={badge.color}>{t('Stufe {tier} von 10', { tier: (badge.tier ?? 0) + 1 })}</span>
      {:else}
        <span>{t('Abzeichen dieser Instanz')}</span>
        {#if badge.granted_at}<span>{t('seit {date}', { date: date.format(new Date(badge.granted_at)) })}</span>{/if}
      {/if}
    </p>
    <div class="box">
      <small>{badge.track ? t('So bekommt man es') : t('Beschreibung')}</small>
      <p>{text}</p>
      {#if badge.track}
        <p class="muted">{t('{name} hat bisher {amount}.', { name, amount: amount(badge.track, badge.hours ?? 0) })}</p>
      {/if}
    </div>
    {#if badge.track && onprogress}
      <button class="btn" onclick={onprogress}>{t('Fortschritt ansehen')}</button>
    {/if}
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 0.55);
  }
  .sheet {
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
