<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { installUpdate, update } from '../lib/update.svelte';
  import Icon from './Icon.svelte';
</script>

{#if update.mode && !update.hidden}
  <div class="update panel row" role="status">
    <Icon name="download" size={16} />
    <span class="grow">
      {#if update.busy}
        {t('update_installing', { percent: update.progress ?? 0 })}
      {:else if update.mode === 'reload'}
        {t('update_web_ready')}
      {:else}
        {t('update_available', { version: update.latest ?? '' })}
      {/if}
    </span>
    {#if !update.busy}
      <button class="btn small primary" onclick={installUpdate}>
        {update.mode === 'reload' ? t('update_reload') : update.mode === 'install' ? t('update_install') : t('update_download')}
      </button>
      <button class="close" onclick={() => (update.hidden = true)} aria-label={t('update_later')} title={t('update_later')}><Icon name="close" size={14} /></button>
    {:else}
      <progress max="100" value={update.progress ?? 0}></progress>
    {/if}
  </div>
{/if}

<style>
  .update {
    gap: 10px;
    padding: 8px 12px;
    margin-bottom: var(--gap);
    font-size: 0.9em;
    border-left: 3px solid var(--accent);
    align-items: center;
  }
  .close {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    color: var(--muted);
  }
  .close:hover {
    color: var(--ink);
    background: var(--line);
  }
  progress {
    width: 140px;
    accent-color: var(--accent);
  }
</style>
