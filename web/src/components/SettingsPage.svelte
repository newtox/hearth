<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  export interface Section {
    id: string;
    name: string;
    group?: string;
    danger?: boolean;
    action?: () => void;
  }

  let { title, sections, current = $bindable(), onclose, children }: { title: string; sections: Section[]; current: string; onclose: () => void; children: Snippet } = $props();

  const groups = $derived.by(() => {
    const out: { name: string; items: Section[] }[] = [];
    for (const s of sections) {
      const name = s.group ?? '';
      let g = out.find((x) => x.name === name);
      if (!g) out.push((g = { name, items: [] }));
      g.items.push(s);
    }
    return out;
  });
  const active = $derived(sections.find((s) => s.id === current));
  let main = $state<HTMLElement>();

  function pick(s: Section) {
    if (s.action) return s.action();
    current = s.id;
    main?.scrollTo({ top: 0 });
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape' && !(e.target as HTMLElement).closest('input, textarea, select') && !document.querySelector('[aria-modal="true"][role="dialog"]:not(.settings)')) onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="settings" role="dialog" aria-modal="true" aria-label={title}>
  <div class="mobilebar">
    <strong class="ellipsis">{title}</strong>
    <button class="icon" onclick={onclose} aria-label={t('common_close')}><Icon name="close" /></button>
  </div>
  <nav class="nav scroll" aria-label={title}>
    <div class="inner">
      <p class="title ellipsis">{title}</p>
      {#each groups as g (g.name)}
        {#if g.name}<h4>{g.name}</h4>{/if}
        {#each g.items as s (s.id)}
          <button class="item" class:on={s.id === current} class:danger={s.danger} aria-current={s.id === current ? 'page' : undefined} onclick={() => pick(s)}>{s.name}</button>
        {/each}
        <hr />
      {/each}
    </div>
  </nav>
  <main class="content scroll" bind:this={main}>
    <div class="inner">
      <h2>{active?.name ?? ''}</h2>
      {@render children()}
    </div>
  </main>
  <button class="close" onclick={onclose} aria-label={t('common_close')}>
    <span class="ring"><Icon name="close" /></span>
    <small>ESC</small>
  </button>
</div>

<style>
  .settings {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    background: var(--bg, var(--panel));
  }
  .nav {
    flex: 1 0 220px;
    display: flex;
    justify-content: flex-end;
    background: var(--panel-2);
  }
  .nav .inner {
    width: 230px;
    padding: 48px 10px 24px 16px;
  }
  .title {
    margin: 0 0 14px 10px;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.05em;
  }
  h4 {
    margin: 4px 10px 6px;
    font-size: 0.78em;
    font-weight: 700;
    color: var(--muted);
  }
  .item {
    display: block;
    width: 100%;
    padding: 7px 10px;
    margin-bottom: 2px;
    border-radius: var(--radius-s);
    text-align: left;
    color: var(--muted);
    font-weight: 500;
  }
  .item:hover {
    background: var(--panel);
    color: var(--ink);
  }
  .item.on {
    background: var(--panel);
    color: var(--ink);
    font-weight: 600;
  }
  .item.danger {
    color: var(--danger);
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 10px 10px;
  }
  hr:last-child {
    display: none;
  }
  .content {
    flex: 1 1 800px;
    min-width: 0;
    background: var(--panel);
  }
  .content .inner {
    max-width: 740px;
    padding: 48px 40px 80px;
  }
  .content h2 {
    margin: 0 0 20px;
  }
  .mobilebar {
    display: none;
  }
  .close {
    position: absolute;
    top: 44px;
    right: max(24px, calc((100vw - 1240px) / 2));
    display: grid;
    justify-items: center;
    gap: 4px;
    color: var(--muted);
  }
  .ring {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: 2px solid currentColor;
  }
  .close:hover {
    color: var(--ink);
  }
  .close small {
    font-size: 0.72em;
    font-weight: 700;
  }
  @media (max-width: 860px) {
    .settings {
      flex-direction: column;
    }
    .nav {
      flex: none;
      justify-content: flex-start;
      overflow-x: auto;
      overflow-y: hidden;
      border-bottom: 1px solid var(--line);
    }
    .nav .inner {
      width: auto;
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 6px 10px 10px;
    }
    .title,
    h4,
    hr {
      display: none;
    }
    .item {
      width: auto;
      white-space: nowrap;
      margin: 0;
    }
    .content .inner {
      padding: 20px 16px 60px;
    }
    .close {
      display: none;
    }
    .mobilebar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      padding: 10px 12px 4px 16px;
      background: var(--panel-2);
      font-family: var(--font-display);
    }
  }
</style>
