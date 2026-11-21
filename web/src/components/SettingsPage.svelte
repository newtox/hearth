<script lang="ts">
  import { backLayer } from '../lib/back.svelte';
  import { t } from '../lib/i18n.svelte';
  import { untrack, type Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { leave } from '../lib/unsaved.svelte';

  export interface Section {
    id: string;
    name: string;
    group?: string;
    danger?: boolean;
    icon?: string;
    action?: () => void;
  }

  let { title, sections, current = $bindable(), onclose, list = true, children }: { title: string; sections: Section[]; current: string; onclose: () => void; list?: boolean; children: Snippet } = $props();

  const narrow = () => matchMedia('(max-width: 860px)').matches;
  let listing = $state(untrack(() => list));

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

  async function pick(s: Section) {
    if (s.id === current && !s.action) {
      listing = false;
      return;
    }
    if (!(await leave())) return;
    if (s.action) return s.action();
    current = s.id;
    listing = false;
    main?.scrollTo({ top: 0 });
  }

  async function close() {
    if (await leave()) onclose();
  }

  async function back() {
    if (narrow() && !listing) {
      if (await leave()) listing = true;
    } else close();
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape' && !(e.target as HTMLElement).closest('input, textarea, select') && !document.querySelector('[aria-modal="true"][role="dialog"]:not(.settings)')) close();
  }
</script>

<svelte:window onkeydown={key} />

<div class="settings" class:listing use:backLayer={back} role="dialog" aria-modal="true" aria-label={title}>
  <div class="mobilebar">
    {#if !listing}<button class="icon" onclick={back} aria-label={t('common_back')}><Icon name="chevron" /></button>{/if}
    <strong class="ellipsis grow">{listing ? title : (active?.name ?? title)}</strong>
    <button class="icon" onclick={close} aria-label={t('common_close')}><Icon name="close" /></button>
  </div>
  <nav class="nav scroll" aria-label={title}>
    <div class="inner">
      <p class="title ellipsis">{title}</p>
      {#each groups as g (g.name)}
        {#if g.name}<h4>{g.name}</h4>{/if}
        <div class="grp">
        {#each g.items as s (s.id)}
          <button class="item" class:on={s.id === current} class:danger={s.danger} aria-current={s.id === current ? 'page' : undefined} onclick={() => pick(s)}>
            {#if s.icon}<span class="ic"><Icon name={s.icon} size={18} /></span>{/if}
            <span class="grow ellipsis">{s.name}</span>
            {#if !s.action}<span class="go"><Icon name="chevron" size={16} /></span>{/if}
          </button>
        {/each}
        </div>
        <hr />
      {/each}
    </div>
  </nav>
  <main class="content scroll" bind:this={main}>
    {#key current}
      <div class="inner view">
        <h2>{active?.name ?? ''}</h2>
        {@render children()}
      </div>
    {/key}
  </main>
  <button class="close" onclick={close} aria-label={t('common_close')}>
    <span class="ring"><Icon name="close" /></span>
    <small>ESC</small>
  </button>
</div>

<style>
  .view {
    animation: rise-in 0.18s ease-out;
  }
  .settings {
    animation: fade-in 0.16s ease-out;
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
    display: flex;
    align-items: center;
    gap: 12px;
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
  .mobilebar,
  .ic,
  .go {
    display: none;
  }
  @media (min-width: 861px) {
    :global([data-style='modern']) .settings {
      gap: 8px;
      padding: 8px;
      background: var(--rail);
    }
    :global([data-style='modern']) .nav,
    :global([data-style='modern']) .content {
      border-radius: var(--radius);
      background: var(--panel);
    }
    :global([data-style='modern']) .content {
      background: var(--chat);
    }
    :global([data-style='modern']) .close {
      top: 36px;
      right: max(32px, calc((100vw - 1240px) / 2));
    }
  }
  :global([data-style='modern']) .item:hover {
    background: var(--panel-2);
  }
  :global([data-style='modern']) .item.on {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--ink);
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
      flex: 1;
      display: none;
      justify-content: flex-start;
      background: var(--bg, var(--panel));
    }
    .listing .nav {
      display: block;
    }
    .listing .content {
      display: none;
    }
    .nav .inner {
      width: auto;
      padding: 8px 14px 40px;
    }
    .title,
    hr {
      display: none;
    }
    h4 {
      margin: 18px 6px 8px;
    }
    .item {
      margin: 0;
      padding: 14px 16px;
      border-radius: 0;
      background: var(--panel);
      color: var(--ink);
      font-weight: 500;
      border-bottom: 1px solid var(--line);
    }
    .grp {
      border-radius: var(--radius);
      overflow: hidden;
    }
    .item:last-child {
      border-bottom: 0;
    }
    .item.on {
      background: var(--panel);
    }
    .ic,
    .go {
      display: grid;
      place-items: center;
      flex: none;
      color: var(--muted);
    }
    .go {
      transform: rotate(-90deg);
    }
    .item.danger .ic {
      color: var(--danger);
    }
    .content .inner {
      padding: 16px 16px 60px;
    }
    .content h2 {
      display: none;
    }
    .close {
      display: none;
    }
    .mobilebar {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 10px 12px;
      background: var(--panel-2);
      font-family: var(--font-display);
      font-size: 1.08em;
    }
    .mobilebar .icon {
      display: grid;
      place-items: center;
      padding: 6px;
      border-radius: var(--radius-s);
      color: var(--muted);
    }
    .mobilebar .icon:first-child:not(:last-child) :global(svg) {
      transform: rotate(90deg);
    }
    :global([data-style='modern']) .item.on {
      background: var(--panel);
    }
  }
</style>
