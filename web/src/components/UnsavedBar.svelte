<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n.svelte';
  import { guard } from '../lib/unsaved.svelte';

  let { dirty, busy = false, guarded = true, onreset, onsave }: { dirty: boolean; busy?: boolean; guarded?: boolean; onreset: () => void; onsave: () => void } = $props();

  onMount(() => (guarded ? guard({ dirty: () => dirty, reset: () => onreset() }) : undefined));
</script>

{#if dirty}
  <div class="unsaved panel" role="status">
    <span class="text">{t('unsaved_bar')}</span>
    <div class="actions">
      <button type="button" class="btn small" onclick={onreset} disabled={busy}>{t('unsaved_reset')}</button>
      <button type="button" class="btn small primary" onclick={onsave} disabled={busy}>{t('common_save_changes')}</button>
    </div>
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
    display: flex;
    flex-wrap: wrap;
  }
  .text {
    flex: 1 1 200px;
    min-width: 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-left: auto;
  }
  @media (max-width: 860px) {
    .text {
      flex-basis: 100%;
    }
    .actions {
      width: 100%;
      margin-left: 0;
    }
    .actions .btn {
      flex: 1 1 0;
      min-width: 0;
      white-space: normal;
    }
  }
</style>
