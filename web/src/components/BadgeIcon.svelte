<script lang="ts">
  import { BADGE_ICONS } from '../lib/badges';
  import type { Badge } from '../lib/store.svelte';

  let { badge, size = 24 }: { badge: Pick<Badge, 'icon' | 'color'>; size?: number } = $props();

  const shape = $derived(badge.icon.startsWith('svg:') ? BADGE_ICONS[badge.icon.slice(4)] : null);

  function fit(svg: SVGSVGElement, _d: string) {
    const apply = () => {
      const path = svg.querySelector('path');
      if (!path) return;
      const b = path.getBBox();
      if (!b.width || !b.height) return;
      const pad = shape?.stroke ? 1.6 : 0.6;
      const side = Math.max(b.width, b.height) + pad * 2;
      svg.setAttribute('viewBox', `${b.x + b.width / 2 - side / 2} ${b.y + b.height / 2 - side / 2} ${side} ${side}`);
    };
    apply();
    return { update: apply };
  }
</script>

{#if shape}
  <svg width={size} height={size} viewBox="0 0 24 24" use:fit={shape.d} style:color={badge.color} fill={shape.stroke ? 'none' : 'currentColor'} stroke={shape.stroke ? 'currentColor' : 'none'} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d={shape.d} />
  </svg>
{:else}
  <span class="glyph" style:width="{size}px" style:height="{size}px" style:font-size="{size * 0.82}px">{badge.icon}</span>
{/if}

<style>
  svg {
    display: block;
    flex: none;
    filter: drop-shadow(0 0 4px color-mix(in srgb, currentColor 40%, transparent));
  }
  .glyph {
    display: inline-grid;
    place-items: center;
    flex: none;
    overflow: hidden;
    line-height: 1;
    font-family: system-ui, 'Noto Color Emoji', sans-serif;
  }
</style>
