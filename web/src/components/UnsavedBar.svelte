<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n.svelte';
  import { guard } from '../lib/unsaved.svelte';

  let { dirty, busy = false, onreset, onsave }: { dirty: boolean; busy?: boolean; onreset: () => void; onsave: () => void } = $props();

  onMount(() => guard({ dirty: () => dirty, reset: () => onreset() }));
</script>

{#if dirty}
  <div class="unsaved panel row" role="status">
    <span class="grow">{t('unsaved_bar')}</span>
    <button type="button" class="btn small" onclick={onreset} disabled={busy}>{t('unsaved_reset')}</button>
    <button type="button" class="btn small primary" onclick={onsave} disabled={busy}>{t('common_save_changes')}</button>
  </div>
{/if}

<style>
  .unsaved {
    position: sticky;
    bottom: 12px;
    z-index: 5;
    gap: 10px;
    align-items: center;
    margin-top: 16px;
    padding: 10px 12px 10px 16px;
    border-left: 3px solid var(--accent);
    box-shadow: var(--panel-shadow);
  }
</style>
