<script lang="ts">
  import { asset } from '../lib/api';
  import { onlyEmoji, parse, type Node } from '../lib/markdown';

  let { text }: { text: string } = $props();

  const nodes = $derived(parse(text));
  const big = $derived(onlyEmoji(nodes));
</script>

{#snippet render(list: Node[])}{#each list as n}{#if n.t === 'text'}{n.v}{:else if n.t === 'bold'}<strong>{@render render(n.c)}</strong>{:else if n.t === 'italic'}<em>{@render render(n.c)}</em>{:else if n.t === 'strike'}<s>{@render render(n.c)}</s>{:else if n.t === 'code'}<code>{n.v}</code>{:else if n.t === 'block'}<pre>{n.v}</pre>{:else if n.t === 'link'}<a href={n.v} target="_blank" rel="noopener noreferrer nofollow">{n.v}</a>{:else if n.t === 'emoji'}<img class="emoji" class:big src={asset(n.file)} alt=":{n.name}:" title=":{n.name}:" />{/if}{/each}{/snippet}

{@render render(nodes)}

<style>
  code,
  pre {
    font-family: ui-monospace, 'Cascadia Code', monospace;
    font-size: 0.9em;
    background: var(--code);
    border-radius: 5px;
  }
  code {
    padding: 1px 5px;
  }
  pre {
    margin: 4px 0;
    padding: 8px 10px;
    overflow-x: auto;
    white-space: pre;
  }
  .emoji {
    width: 1.4em;
    height: 1.4em;
    vertical-align: -0.3em;
    object-fit: contain;
  }
  .emoji.big {
    width: 3em;
    height: 3em;
  }
</style>
