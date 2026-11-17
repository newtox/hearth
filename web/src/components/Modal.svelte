<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let { title, onclose, wide = false, children }: { title: string; onclose: () => void; wide?: boolean; children: Snippet } = $props();

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" onclick={(e) => e.target === e.currentTarget && onclose()} role="presentation">
  <div class="panel dialog" class:wide role="dialog" aria-modal="true" aria-label={title}>
    <header class="row">
      <h2 class="grow">{title}</h2>
      <button class="x" onclick={onclose} aria-label={t('common_close')}><Icon name="close" /></button>
    </header>
    <div class="body scroll">
      {@render children()}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    padding: 16px;
    grid-template-columns: minmax(0, 1fr);
    justify-items: center;
    background: rgba(8, 5, 20, 0.6);
    backdrop-filter: blur(3px);
  }
  .dialog {
    width: min(460px, 100%);
    max-height: min(92dvh, 760px);
    display: flex;
    flex-direction: column;
  }
  .dialog.wide {
    width: min(860px, 100%);
    height: min(92dvh, 680px);
  }
  header {
    padding: 18px 20px 12px;
    align-items: flex-start;
  }
  header h2 {
    overflow-wrap: anywhere;
  }
  .x {
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .x:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .body {
    padding: 4px 20px 20px;
    flex: 1;
    min-height: 0;
  }
</style>
