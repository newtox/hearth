<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { thumb } from '../lib/api';
  import { frame, hoverHost, isAnimated, sizeFor } from '../lib/still.svelte';

  let { id, name, image = null, size = 36, online = undefined, status = undefined, square = false }: { id: string; name: string; image?: string | null; size?: number; online?: boolean; status?: string; square?: boolean } = $props();

  const LABELS: Record<string, string> = { online: 'common_online', idle: 'common_idle', dnd: 'common_do_not_disturb', invisible: 'common_invisible', offline: 'common_offline' };
  let hovered = $state(false);
  const url = $derived(thumb(image, sizeFor(size)));
  const moving = $derived(isAnimated(url));
  const shown = $derived(!url ? null : !moving || hovered ? url : frame(url, sizeFor(size)));
  const dotState = $derived(status ?? (online === undefined ? undefined : online ? 'online' : 'offline'));

  const hue = $derived([...id].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 360, 7));
  const initials = $derived(
    name
      .split(/\s+/)
      .slice(0, 2)
      .map((p) => [...p][0] ?? '')
      .join('')
      .toUpperCase(),
  );
</script>

<span class="avatar" class:square style="--s:{size}px; --h:{hue}" use:hoverHost={(on) => (hovered = on)}>
  {#if image && shown}
    <img src={shown} alt="" loading="lazy" />
  {:else if image}
    <span class="blank"></span>
  {:else}
    <span class="initials">{initials}</span>
  {/if}
  {#if dotState}
    <span class="dot {dotState}" title={t(LABELS[dotState] ?? dotState)} role="img" aria-label={t(LABELS[dotState] ?? dotState)}></span>
  {/if}
</span>

<style>
  .avatar {
    position: relative;
    flex: none;
    width: var(--s);
    height: var(--s);
    display: inline-grid;
    place-items: center;
    border-radius: 50%;
    background: hsl(var(--h) 45% 42%);
    color: #fff;
    font-weight: 700;
    font-size: calc(var(--s) * 0.4);
    font-family: var(--font-display);
  }
  .square {
    border-radius: calc(var(--s) * 0.28);
  }
  :global([data-style='modern']) .avatar {
    border-radius: calc(var(--s) * 0.34);
  }
  :global([data-theme='pergament']) .avatar {
    border-radius: 3px;
    box-shadow: 0 0 0 2px #9a7636;
    background: hsl(var(--h) 30% 32%);
  }
  .blank {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    background: var(--panel-2);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: inherit;
  }
  .dot {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 30%;
    height: 30%;
    min-width: 9px;
    min-height: 9px;
    border-radius: 50%;
    background: var(--muted);
    border: 2px solid var(--panel);
  }
  .dot.online {
    background: var(--ok);
  }
  .dot.idle {
    background: #f0b232;
  }
  .dot.dnd {
    background: var(--danger);
  }
  .dot.dnd::after {
    content: '';
    position: absolute;
    left: 22%;
    right: 22%;
    top: calc(50% - 1px);
    height: 2px;
    border-radius: 1px;
    background: var(--panel);
  }
  .dot.invisible {
    background: var(--panel);
    box-shadow: inset 0 0 0 2px var(--muted);
  }
</style>
