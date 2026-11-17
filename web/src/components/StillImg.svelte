<script lang="ts">
  import { hoverHost, isAnimated, still } from '../lib/still.svelte';

  let { src: original, alt = '' }: { src: string; alt?: string } = $props();
  const src = $derived(original.replace(/\/uploads\/([a-z0-9]+\.[a-z0-9]+)$/i, '/thumbs/128/$1'));
  let moving = $state(false);
  const shown = $derived(moving || !isAnimated(src) ? src : still(src));
</script>

<img src={shown ?? undefined} {alt} use:hoverHost={(on) => (moving = on)} />
