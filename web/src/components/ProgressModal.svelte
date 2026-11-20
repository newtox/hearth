<script lang="ts">
  import { untrack } from 'svelte';
  import { TRACKS, amount } from '../lib/activity';
  import { t } from '../lib/i18n.svelte';
  import type { Progress } from '../lib/store.svelte';
  import BadgeIcon from './BadgeIcon.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { progress, track: initial, own, name, onclose }: { progress: Progress; track: string; own: boolean; name: string; onclose: () => void } = $props();

  let track = $state(untrack(() => (progress.tracks.some((x) => x.track === initial) ? initial : (progress.tracks[0]?.track ?? initial))));
  const current = $derived(progress.tracks.find((x) => x.track === track) ?? progress.tracks[0]);
  const tier = $derived(current.tier ?? -1);
  const next = $derived(tier + 1 < current.steps.length ? tier + 1 : null);
  const from = $derived(tier >= 0 ? current.steps[tier] : 0);
  const share = $derived(next === null ? 1 : Math.min(1, Math.max(0, (current.value - from) / (current.steps[next] - from))));
</script>

<Modal title={own ? t('progress_modal_progress') : t('progress_modal_progress_name', { name })} {onclose}>
  <div class="stack">
    <div class="tabs" role="tablist">
      {#each progress.tracks as p (p.track)}
        <button role="tab" aria-selected={track === p.track} class:on={track === p.track} onclick={() => (track = p.track)}>
          <BadgeIcon badge={{ icon: p.icon, color: p.tier === null ? 'var(--muted)' : progress.colors[p.tier] }} size={20} />
          {TRACKS[p.track] ? t(TRACKS[p.track].name) : p.track}
        </button>
      {/each}
    </div>

    <div class="now">
      <BadgeIcon badge={{ icon: current.icon, color: tier >= 0 ? progress.colors[tier] : 'var(--muted)' }} size={40} />
      <div class="grow">
        <strong>
          {#if next === null}{t('progress_modal_highest_level_reached')}{:else}{t('progress_modal_amount_go_until_level_tier', { amount: amount(track, current.steps[next] - current.value), tier: next + 1 })}{/if}
        </strong>
        <div class="bar" role="progressbar" aria-valuenow={Math.round(share * 100)} aria-valuemin="0" aria-valuemax="100"><i style:width="{share * 100}%"></i></div>
        <small class="muted">{t('progress_modal_so_far_amount', { amount: amount(track, current.value) })}</small>
      </div>
      {#if next !== null}<BadgeIcon badge={{ icon: current.icon, color: progress.colors[next] }} size={40} />{/if}
    </div>

    {#if TRACKS[track]}<p class="muted">{t(TRACKS[track].hint)}</p>{/if}

    <ol class="levels">
      {#each current.steps as step, i}
        <li class:locked={i > tier} class:here={i === tier}>
          <BadgeIcon badge={{ icon: current.icon, color: progress.colors[i] }} size={30} />
          <strong>{t('progress_modal_level_n', { n: i + 1 })}</strong>
          <span class="muted">{#if i > tier}<Icon name="lock" size={12} />{/if}{amount(track, step)}{i === current.steps.length - 1 ? '+' : ''}</span>
        </li>
      {/each}
    </ol>
  </div>
</Modal>

<style>
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
    color: var(--muted);
    font-weight: 600;
    font-size: 0.9em;
  }
  .tabs button.on {
    color: var(--ink);
    border-color: var(--accent);
    background: var(--panel-2);
  }
  .now {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  .now .grow {
    display: grid;
    gap: 6px;
  }
  .bar {
    height: 10px;
    border-radius: 99px;
    background: color-mix(in srgb, var(--ink) 14%, transparent);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    border-radius: 99px;
    background: var(--accent);
  }
  .levels {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 14px 8px;
  }
  .levels li {
    display: grid;
    justify-items: center;
    gap: 3px;
    text-align: center;
    padding: 8px 4px;
    border-radius: var(--radius-s);
    border: 1px solid transparent;
  }
  .levels li.here {
    border-color: var(--line);
    background: var(--panel-2);
  }
  .levels li.locked {
    opacity: 0.45;
  }
  .levels span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.88em;
  }
</style>
