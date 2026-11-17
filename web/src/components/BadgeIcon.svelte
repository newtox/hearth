<script lang="ts">
  import { BADGE_ICONS } from '../lib/badges';
  import type { Badge } from '../lib/store.svelte';

  let { badge, size = 24 }: { badge: Pick<Badge, 'icon' | 'color'>; size?: number } = $props();

  const shape = $derived(badge.icon.startsWith('svg:') ? BADGE_ICONS[badge.icon.slice(4)] : null);
</script>

{#if shape}
  <svg width={size} height={size} viewBox="0 0 24 24" style:color={badge.color} fill={shape.stroke ? 'none' : 'currentColor'} stroke={shape.stroke ? 'currentColor' : 'none'} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d={shape.d} />
  </svg>
{:else}
  <span class="glyph" style:font-size="{size * 0.9}px">{badge.icon}</span>
{/if}

<style>
  svg {
    display: block;
    flex: none;
    filter: drop-shadow(0 0 4px color-mix(in srgb, currentColor 40%, transparent));
  }
  .glyph {
    line-height: 1;
    font-family: system-ui, 'Noto Color Emoji', sans-serif;
  }
</style>
