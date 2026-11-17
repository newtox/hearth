<script lang="ts">
  import { hoverHost, isAnimated, still } from '../lib/still.svelte';

  let { src, alt = '' }: { src: string; alt?: string } = $props();
  let moving = $state(false);
  const shown = $derived(moving || !isAnimated(src) ? src : still(src));
</script>

<img src={shown ?? undefined} {alt} use:hoverHost={(on) => (moving = on)} />
