<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { asset } from '../lib/api';
  import { app, guild, type Emoji } from '../lib/store.svelte';
  import { UNICODE } from '../lib/emojis';

  let { stickers = false, external = true, onpick, onclose }: { stickers?: boolean; external?: boolean; onpick: (value: string, kind: 'unicode' | 'custom' | 'sticker', emoji?: Emoji) => void; onclose: () => void } = $props();

  let tab = $state<'emoji' | 'sticker'>('emoji');
  const sorted = $derived([...app.guilds].filter((x) => external || x.id === guild()?.id).sort((a, b) => Number(b.id === guild()?.id) - Number(a.id === guild()?.id)));
  const custom = $derived(sorted.flatMap((g) => g.emojis.filter((e) => e.kind === 'emoji')));
  const stickerList = $derived(sorted.flatMap((g) => g.emojis.filter((e) => e.kind === 'sticker')));
  const query = $state({ text: '' });
  const q = $derived(query.text.trim().toLowerCase().replace(/^:|:$/g, ''));
  function rank(words: string[]) {
    if (words.includes(q)) return 0;
    if (words.some((x) => x.startsWith(q))) return 1;
    return words.some((x) => x.includes(q)) ? 2 : 3;
  }
  const shownUnicode = $derived(
    q
      ? UNICODE.map((e) => ({ e, r: rank(e.words) }))
          .filter((x) => x.r < 3)
          .sort((a, b) => a.r - b.r)
          .map((x) => x.e)
      : UNICODE,
  );
  const shownCustom = $derived(q ? custom.filter((e) => e.name.toLowerCase().includes(q)) : custom);
  const shownStickers = $derived(q ? stickerList.filter((e) => e.name.toLowerCase().includes(q)) : stickerList);
  let search = $state<HTMLInputElement>();
  $effect(() => {
    search?.focus();
  });

  function enter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    if (tab === 'sticker') {
      const s = shownStickers[0];
      if (s) onpick(`<sticker:${s.name}:${s.file}>`, 'sticker', s);
    } else if (shownCustom[0]) onpick(`<:${shownCustom[0].name}:${shownCustom[0].file}>`, 'custom', shownCustom[0]);
    else if (shownUnicode[0]) onpick(shownUnicode[0].glyph, 'unicode');
  }

</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="veil" onclick={onclose} role="presentation"></div>
<div class="panel picker" role="dialog" aria-label={t('Emoji auswählen')}>
  {#if stickers}
    <div class="row tabs">
      <button class:on={tab === 'emoji'} onclick={() => (tab = 'emoji')}>{t('Emojis')}</button>
      <button class:on={tab === 'sticker'} onclick={() => (tab = 'sticker')}>{t('Sticker')}</button>
    </div>
  {/if}
  <input class="input search" bind:this={search} bind:value={query.text} onkeydown={enter} placeholder={tab === 'sticker' ? t('Sticker suchen') : t('Emoji suchen')} aria-label={tab === 'sticker' ? t('Sticker suchen') : t('Emoji suchen')} />
  <div class="scroll body">
    {#if tab === 'emoji'}
      {#if shownCustom.length}
        <div class="grid">
          {#each shownCustom as e (e.id)}
            <button onclick={() => onpick(`<:${e.name}:${e.file}>`, 'custom', e)} title=":{e.name}:"><img src={asset(e.file)} alt=":{e.name}:" /></button>
          {/each}
        </div>
        {#if shownUnicode.length}<hr />{/if}
      {/if}
      <div class="grid">
        {#each shownUnicode as e (e.glyph)}
          <button onclick={() => onpick(e.glyph, 'unicode')} title=":{e.words[0]}:">{e.glyph}</button>
        {/each}
      </div>
      {#if !shownCustom.length && !shownUnicode.length}<p class="muted">{t('Nichts gefunden für „{q}“', { q: query.text })}</p>{/if}
    {:else}
      <div class="grid big">
        {#each shownStickers as e (e.id)}
          <button onclick={() => onpick(`<sticker:${e.name}:${e.file}>`, 'sticker', e)} title={e.name}><img src={asset(e.file)} alt={e.name} /></button>
        {:else}
          <p class="muted">{q ? t('Nichts gefunden für „{q}“', { q: query.text }) : t('Noch keine Sticker. In den Servereinstellungen unter „Emojis“ lassen sich welche hochladen.')}</p>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 30;
  }
  .picker {
    position: absolute;
    right: 12px;
    bottom: 74px;
    z-index: 31;
    width: min(340px, calc(100% - 24px));
    border-radius: var(--radius) !important;
    border: var(--panel-border) !important;
    box-shadow: var(--panel-shadow) !important;
    background: var(--panel-2);
  }
  .tabs {
    padding: 8px 10px 0;
  }
  .tabs button {
    padding: 4px 12px;
    border-radius: var(--radius-s);
    color: var(--muted);
    font-weight: 600;
  }
  .tabs button.on {
    background: var(--panel);
    color: var(--ink);
  }
  .search {
    margin: 10px 10px 0;
    width: calc(100% - 20px);
  }
  .body {
    max-height: 260px;
    padding: 10px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 2px;
  }
  .grid.big {
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .grid.big p {
    grid-column: 1 / -1;
  }
  .grid button {
    aspect-ratio: 1;
    font-size: 1.35rem;
    border-radius: 8px;
    display: grid;
    place-items: center;
    font-family: system-ui, 'Noto Color Emoji', sans-serif;
  }
  .grid button:hover {
    background: var(--panel);
  }
  img {
    width: 78%;
    height: 78%;
    object-fit: contain;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 8px 0;
  }
</style>
