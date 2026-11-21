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

  function area(el: HTMLElement) {
    const frame = el.closest('.framed');
    return frame ? frame.getBoundingClientRect() : { left: 0, top: 0, width: innerWidth, height: innerHeight };
  }

  function place(el: HTMLElement, x: number, y: number) {
    const r = el.getBoundingClientRect();
    const a = area(el);
    return { x: Math.max(8, Math.min(x - a.left, a.width - r.width - 8)), y: Math.max(8, Math.min(y - a.top, a.height - r.height - 8)) };
  }

  $effect(() => {
    if (!menu) return;
    pos = { x: menu.x, y: menu.y };
    open = null;
    tick().then(() => {
      if (!root) return;
      pos = place(root, menu.x, menu.y);
      first(root)?.focus({ preventScroll: true });
    });
  });

  function first(el: HTMLElement | undefined) {
    return el?.querySelector<HTMLButtonElement>('button:not(:disabled)');
  }

  async function showSub(i: number, button: HTMLElement, focus = false) {
    if (open === i) {
      if (focus) first(side)?.focus({ preventScroll: true });
      return;
    }
    open = i;
    await tick();
    if (!side) return;
    const b = button.getBoundingClientRect();
    const r = side.getBoundingClientRect();
    const x = b.right + r.width + 6 > innerWidth ? b.left - r.width - 6 : b.right + 6;
    subPos = place(side, x, b.top - 6);
    if (focus) first(side)?.focus({ preventScroll: true });
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
    const list = [...((inSub ? side : root)?.querySelectorAll<HTMLElement>('button:not(:disabled), input') ?? [])];
    const i = list.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const n = list.length;
      list[(i + (e.key === 'ArrowDown' ? 1 : -1) + n) % n]?.focus({ preventScroll: true });
    } else if (e.key === 'ArrowLeft' && inSub) {
      e.preventDefault();
      const parent = root?.querySelector<HTMLButtonElement>(`[data-index="${open}"]`);
      open = null;
      parent?.focus({ preventScroll: true });
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
  {:else if item.range}
    {@const range = item.range}
    <label class="range">
      <span class="row"><span class="grow">{item.label}</span><span class="muted">{Math.round(range.value() * 100)} %</span></span>
      <input type="range" min="0" max={range.max} step="0.01" value={range.value()} oninput={(e) => range.set(Number(e.currentTarget.value))} ondblclick={() => range.set(1)} />
    </label>
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
      <span class="text">{item.label}{#if item.hint}<small>{item.hint}</small>{/if}</span>
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
  <div class="veil" role="presentation"></div>
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
  .range {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    min-width: 200px;
  }
  .range .row {
    display: flex;
    gap: 8px;
    font-size: 0.92em;
  }
  .range .grow {
    flex: 1;
  }
  .range input {
    width: 100%;
    accent-color: var(--accent);
  }
  .text small {
    display: block;
    margin-top: 2px;
    font-size: 0.82em;
    color: var(--muted);
    white-space: normal;
  }
  .menu {
    animation: pop-in 0.12s ease-out;
    transform-origin: top left;
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
  .item:focus-visible,
  .item.open {
    background: var(--accent-fill);
    color: var(--accent-ink);
    outline: none;
  }
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
  .item.danger:focus-visible {
    background: var(--danger);
    color: #fff;
  }
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
  .item:focus-visible .box {
    border-color: currentColor;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 5px 4px;
  }
  @media (hover: hover) {
    .item:hover:not(:disabled) {
      background: var(--accent-fill);
      color: var(--accent-ink);
    }
    .item:hover:not(:disabled) :global(svg) {
      color: inherit;
    }
    .item.danger:hover:not(:disabled) {
      background: var(--danger);
      color: #fff;
    }
    .item.danger:hover:not(:disabled) :global(svg) {
      color: #fff;
    }
    .item:hover:not(:disabled) .box {
      border-color: currentColor;
    }
  }
  .veil {
    display: none;
  }
  @media (max-width: 860px) {
    .veil {
      display: block;
      position: fixed;
      inset: 0;
      z-index: 79;
      background: rgb(0 0 0 / 0.45);
      animation: fade-in 0.15s ease-out;
    }
    .menu {
      left: 0 !important;
      right: 0;
      top: auto !important;
      bottom: 0;
      min-width: 0;
      max-width: none;
      max-height: 75dvh;
      padding: 8px 10px max(14px, env(safe-area-inset-bottom));
      border-radius: calc(var(--radius) + 6px) calc(var(--radius) + 6px) 0 0;
      transform-origin: bottom center;
      animation: sheet-up 0.2s cubic-bezier(0.2, 0.9, 0.3, 1);
    }
    .menu::before {
      content: '';
      display: block;
      width: 40px;
      height: 4px;
      margin: 2px auto 10px;
      border-radius: 99px;
      background: var(--line);
    }
    .sub {
      z-index: 81;
    }
    .item {
      padding: 13px 12px;
      font-size: 1em;
      gap: 14px;
    }
    hr {
      margin: 6px 2px;
    }
  }
  @keyframes sheet-up {
    from {
      transform: translateY(40%);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .menu {
      animation: none;
    }
  }
</style>
