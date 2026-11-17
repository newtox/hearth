<script lang="ts">
  import { tick } from 'svelte';
  import { app, type MenuItem } from '../lib/store.svelte';
  import Icon from './Icon.svelte';

  type Item = Exclude<MenuItem, 'sep'>;

  let root = $state<HTMLDivElement>();
  let side = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });
  let open = $state<number | null>(null);
  let subPos = $state({ x: 0, y: 0 });

  const menu = $derived(app.menu);
  const subItems = $derived(open !== null && menu ? ((menu.items[open] as Item).items ?? []) : []);

  const flag = (f: boolean | (() => boolean) | undefined) => (typeof f === 'function' ? f() : !!f);

  function close() {
    app.menu = null;
    open = null;
  }

  function place(el: HTMLElement, x: number, y: number) {
    const r = el.getBoundingClientRect();
    return { x: Math.max(8, Math.min(x, innerWidth - r.width - 8)), y: Math.max(8, Math.min(y, innerHeight - r.height - 8)) };
  }

  $effect(() => {
    if (!menu) return;
    pos = { x: menu.x, y: menu.y };
    open = null;
    tick().then(() => {
      if (!root) return;
      pos = place(root, menu.x, menu.y);
      first(root)?.focus();
    });
  });

  function first(el: HTMLElement | undefined) {
    return el?.querySelector<HTMLButtonElement>('button:not(:disabled)');
  }

  async function showSub(i: number, button: HTMLElement, focus = false) {
    if (open === i) {
      if (focus) first(side)?.focus();
      return;
    }
    open = i;
    await tick();
    if (!side) return;
    const b = button.getBoundingClientRect();
    const r = side.getBoundingClientRect();
    const x = b.right + r.width + 6 > innerWidth ? b.left - r.width - 6 : b.right + 6;
    subPos = place(side, x, b.top - 6);
    if (focus) first(side)?.focus();
  }

  function run(item: Item, i: number | null, e: MouseEvent | KeyboardEvent) {
    if (flag(item.disabled)) return;
    if (item.items && i !== null) {
      showSub(i, e.currentTarget as HTMLElement, true);
      return;
    }
    item.action?.();
    if (!item.keep) close();
  }

  function keys(e: KeyboardEvent, inSub: boolean) {
    const list = [...((inSub ? side : root)?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])];
    const i = list.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const n = list.length;
      list[(i + (e.key === 'ArrowDown' ? 1 : -1) + n) % n]?.focus();
    } else if (e.key === 'ArrowLeft' && inSub) {
      e.preventDefault();
      const parent = root?.querySelector<HTMLButtonElement>(`[data-index="${open}"]`);
      open = null;
      parent?.focus();
    } else if (e.key === 'ArrowRight' && !inSub) {
      const el = document.activeElement as HTMLElement;
      const idx = Number(el?.dataset.index);
      if (menu && (menu.items[idx] as Item)?.items) {
        e.preventDefault();
        showSub(idx, el, true);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'Tab') {
      e.preventDefault();
    }
  }

  function outside(e: PointerEvent) {
    if (!menu) return;
    const target = e.target as Node;
    if (root?.contains(target) || side?.contains(target)) return;
    close();
  }
</script>

<svelte:window onpointerdown={outside} onresize={close} onblur={close} onwheel={(e) => !root?.contains(e.target as Node) && !side?.contains(e.target as Node) && close()} />

{#snippet entry(item: MenuItem, i: number, inSub: boolean)}
  {#if item === 'sep'}
    <hr />
  {:else}
    {@const checkable = item.checked !== undefined}
    <button
      class="item"
      class:danger={item.danger}
      class:open={!inSub && open === i}
      data-index={i}
      role={checkable ? 'menuitemcheckbox' : 'menuitem'}
      aria-checked={checkable ? flag(item.checked) : undefined}
      aria-haspopup={item.items ? 'menu' : undefined}
      aria-expanded={item.items ? open === i : undefined}
      disabled={flag(item.disabled)}
      onclick={(e) => run(item, inSub ? null : i, e)}
      onpointerenter={(e) => !inSub && (item.items ? showSub(i, e.currentTarget) : (open = null))}
    >
      {#if item.color !== undefined}<span class="dot" style:background={item.color ?? 'var(--muted)'}></span>{/if}
      <span class="text">{item.label}</span>
      {#if checkable}
        <span class="box" class:on={flag(item.checked)}>{#if flag(item.checked)}<Icon name="check" size={13} />{/if}</span>
      {:else if item.items}
        <Icon name="right" size={15} />
      {:else if item.icon}
        <Icon name={item.icon} size={16} />
      {/if}
    </button>
  {/if}
{/snippet}

{#if menu}
  <div class="menu panel" role="menu" tabindex="-1" bind:this={root} style:left="{pos.x}px" style:top="{pos.y}px" onkeydown={(e) => keys(e, false)} oncontextmenu={(e) => e.preventDefault()}>
    {#each menu.items as item, i}{@render entry(item, i, false)}{/each}
  </div>
  {#if open !== null && subItems.length}
    <div class="menu panel sub" role="menu" tabindex="-1" bind:this={side} style:left="{subPos.x}px" style:top="{subPos.y}px" onkeydown={(e) => keys(e, true)} oncontextmenu={(e) => e.preventDefault()}>
      {#each subItems as item, i}{@render entry(item, i, true)}{/each}
    </div>
  {/if}
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 80;
    min-width: 220px;
    max-width: 300px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 6px;
    background: var(--panel-2);
    border: var(--panel-border);
    border-radius: var(--radius);
    box-shadow: var(--panel-shadow), 0 12px 32px rgb(0 0 0 / 0.35);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 9px;
    border-radius: var(--radius-s);
    text-align: left;
    color: var(--ink);
    font-size: 0.93em;
  }
  .item :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item:hover:not(:disabled),
  .item:focus-visible,
  .item.open {
    background: var(--accent-fill);
    color: var(--accent-ink);
    outline: none;
  }
  .item:hover:not(:disabled) :global(svg),
  .item:focus-visible :global(svg),
  .item.open :global(svg) {
    color: inherit;
  }
  .item.danger {
    color: var(--danger);
  }
  .item.danger :global(svg) {
    color: var(--danger);
  }
  .item.danger:hover:not(:disabled),
  .item.danger:focus-visible {
    background: var(--danger);
    color: #fff;
  }
  .item.danger:hover:not(:disabled) :global(svg),
  .item.danger:focus-visible :global(svg) {
    color: #fff;
  }
  .item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .dot {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    flex: none;
  }
  .box {
    width: 18px;
    height: 18px;
    flex: none;
    display: grid;
    place-items: center;
    border: 1.5px solid var(--muted);
    border-radius: 5px;
  }
  .box.on {
    background: var(--accent-fill);
    border-color: var(--accent-fill);
    color: var(--accent-ink);
  }
  .box.on :global(svg) {
    color: var(--accent-ink);
  }
  .item:hover:not(:disabled) .box,
  .item:focus-visible .box {
    border-color: currentColor;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 5px 4px;
  }
</style>
