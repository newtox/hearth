<script lang="ts">
  import { asset } from '../lib/api';
  import { onlyEmoji, parse, type Node } from '../lib/markdown';
  import { app, guild } from '../lib/store.svelte';

  function person(name: string) {
    return guild()?.members.find((m) => m.username === name) ?? app.dms.flatMap((d) => d.recipients ?? []).find((u) => u.username === name) ?? (app.me?.username === name ? app.me : null);
  }

  let { text }: { text: string } = $props();

  const nodes = $derived(parse(text));
  const big = $derived(onlyEmoji(nodes));
</script>

{#snippet render(list: Node[])}{#each list as n}{#if n.t === 'text'}{n.v}{:else if n.t === 'bold'}<strong>{@render render(n.c)}</strong>{:else if n.t === 'italic'}<em>{@render render(n.c)}</em>{:else if n.t === 'strike'}<s>{@render render(n.c)}</s>{:else if n.t === 'code'}<code>{n.v}</code>{:else if n.t === 'block'}<pre>{n.v}</pre>{:else if n.t === 'link'}<a href={n.v} target="_blank" rel="noopener noreferrer nofollow">{n.v}</a>{:else if n.t === 'mention'}{@const who = n.v === 'everyone' ? null : person(n.v)}{#if n.v === 'everyone'}<span class="mention">@everyone</span>{:else if who}<button class="mention" class:me={who.id === app.me?.id} onclick={() => (app.profile = who.id)}>@{'nickname' in who && who.nickname ? who.nickname : who.display_name}</button>{:else}@{n.v}{/if}{:else if n.t === 'emoji'}<img class="emoji" class:big src={asset(n.file)} alt=":{n.name}:" title=":{n.name}:" />{/if}{/each}{/snippet}

{@render render(nodes)}

<style>
  .mention {
    display: inline;
    padding: 0 3px;
    border-radius: 4px;
    font: inherit;
    font-weight: 600;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  button.mention:hover {
    background: color-mix(in srgb, var(--accent) 30%, transparent);
  }
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
