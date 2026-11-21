<script lang="ts" module>
  type Entry = [glyph: string, code: string, label: string, search: string];
  type Category = [id: string, emojis: Entry[]];

  const LOADERS = {
    de: () => import('../lib/emoji/de.json'),
    en: () => import('../lib/emoji/en.json'),
    da: () => import('../lib/emoji/da.json'),
  };

  let cache: Promise<Category[]> | null = null;
</script>

<script lang="ts">
  import { backLayer } from '../lib/back.svelte';
  import { tick } from 'svelte';
  import { t, lang } from '../lib/i18n.svelte';
  import { asset } from '../lib/api';
  import { app, guild, type Emoji } from '../lib/store.svelte';
  import Icon from './Icon.svelte';
  import Avatar from './Avatar.svelte';

  let { stickers = false, external = true, onpick, onclose }: { stickers?: boolean; external?: boolean; onpick: (value: string, kind: 'unicode' | 'custom' | 'sticker', emoji?: Emoji) => void; onclose: () => void } = $props();

  const ICONS: Record<string, string> = { people: 'smile', nature: 'leaf', food: 'food', activities: 'gamepad', travel: 'bike', objects: 'bulb', symbols: 'heart', flags: 'flag' };

  let tab = $state<'emoji' | 'sticker'>('emoji');
  let categories = $state.raw<Category[]>([]);
  cache ??= LOADERS[lang]().then((m) => m.default as Category[]);
  cache.then((c) => (categories = c));

  const sorted = $derived([...app.guilds].filter((x) => external || x.id === guild()?.id).sort((a, b) => Number(b.id === guild()?.id) - Number(a.id === guild()?.id)));
  const servers = $derived(sorted.map((g) => ({ g, emojis: g.emojis.filter((e) => e.kind === 'emoji') })).filter((x) => x.emojis.length));
  const custom = $derived(servers.flatMap((x) => x.emojis));
  const stickerList = $derived(sorted.flatMap((g) => g.emojis.filter((e) => e.kind === 'sticker')));
  const flat = $derived(categories.flatMap(([, list]) => list.map((e) => ({ e, words: e[3].split(' ') }))));

  const query = $state({ text: '' });
  const q = $derived(query.text.trim().toLowerCase().replace(/^:|:$/g, ''));
  function rank(code: string, label: string, words: string[]) {
    if (code === q) return 0;
    if (label.toLowerCase().startsWith(q)) return 1;
    if (words.includes(q)) return 2;
    if (words.some((x) => x.startsWith(q))) return 3;
    return words.some((x) => x.includes(q)) ? 4 : 5;
  }
  const shownUnicode = $derived(
    q
      ? flat
          .map((x) => ({ e: x.e, r: rank(x.e[1], x.e[2], x.words) }))
          .filter((x) => x.r < 5)
          .sort((a, b) => a.r - b.r)
          .map((x) => x.e)
      : [],
  );
  const shownCustom = $derived(q ? custom.filter((e) => e.name.toLowerCase().includes(q)) : custom);
  const shownStickers = $derived(q ? stickerList.filter((e) => e.name.toLowerCase().includes(q)) : stickerList);

  let search = $state<HTMLInputElement>();
  let body = $state<HTMLDivElement>();
  let active = $state('');
  let hover = $state<{ glyph?: string; file?: string; name: string } | null>(null);

  $effect(() => {
    search?.focus();
  });

  $effect(() => {
    if (!active && !q) active = servers[0] ? `g-${servers[0].g.id}` : (categories[0]?.[0] ?? '');
  });

  function sections() {
    return body ? [...body.querySelectorAll<HTMLElement>('[data-section]')] : [];
  }

  function spy() {
    if (!body || q) return;
    const top = body.scrollTop + 8;
    let current = sections()[0]?.dataset.section ?? '';
    for (const s of sections()) if (s.offsetTop <= top) current = s.dataset.section ?? current;
    active = current;
  }

  async function jump(id: string) {
    if (query.text) {
      query.text = '';
      await tick();
    }
    const el = sections().find((s) => s.dataset.section === id);
    if (body && el) body.scrollTo({ top: el.offsetTop });
    active = id;
  }

  function enter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    if (tab === 'sticker') {
      const s = shownStickers[0];
      if (s) onpick(`<sticker:${s.name}:${s.file}>`, 'sticker', s);
    } else if (shownCustom[0] && q) onpick(`<:${shownCustom[0].name}:${shownCustom[0].file}>`, 'custom', shownCustom[0]);
    else if (shownUnicode[0]) onpick(shownUnicode[0][0], 'unicode');
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="veil" use:backLayer={onclose} onclick={onclose} role="presentation"></div>
<div class="panel picker" role="dialog" aria-label={t('emoji_picker_pick_emoji')}>
  {#if stickers}
    <div class="row tabs">
      <button class:on={tab === 'emoji'} onclick={() => (tab = 'emoji')}>{t('common_emojis')}</button>
      <button class:on={tab === 'sticker'} onclick={() => (tab = 'sticker')}>{t('common_stickers')}</button>
    </div>
  {/if}
  <input class="input search" bind:this={search} bind:value={query.text} onkeydown={enter} placeholder={tab === 'sticker' ? t('emoji_picker_search_stickers') : t('emoji_picker_search_emoji')} aria-label={tab === 'sticker' ? t('emoji_picker_search_stickers') : t('emoji_picker_search_emoji')} />
  <div class="layout">
    {#if tab === 'emoji'}
      <nav class="rail" class:dim={!!q}>
        {#each servers as x (x.g.id)}
          <button class:on={!q && active === `g-${x.g.id}`} onclick={() => jump(`g-${x.g.id}`)} title={x.g.name} aria-label={x.g.name}>
            <Avatar id={x.g.id} name={x.g.name} image={x.g.icon} size={22} square />
          </button>
        {/each}
        {#if servers.length}<span class="sep"></span>{/if}
        {#each categories as [id] (id)}
          <button class:on={!q && active === id} onclick={() => jump(id)} title={t(`emoji_picker_cat_${id}`)} aria-label={t(`emoji_picker_cat_${id}`)}>
            <Icon name={ICONS[id]} size={18} />
          </button>
        {/each}
      </nav>
    {/if}
    <div class="scroll body" bind:this={body} onscroll={spy}>
      {#if tab === 'emoji'}
        {#if q}
          {#if shownCustom.length}
            <div class="grid">
              {#each shownCustom as e (e.id)}
                <button onclick={() => onpick(`<:${e.name}:${e.file}>`, 'custom', e)} onmouseenter={() => (hover = { file: e.file, name: e.name })} title=":{e.name}:"><img src={asset(e.file)} alt=":{e.name}:" /></button>
              {/each}
            </div>
            {#if shownUnicode.length}<hr />{/if}
          {/if}
          <div class="grid">
            {#each shownUnicode as e (e[0])}
              <button onclick={() => onpick(e[0], 'unicode')} onmouseenter={() => (hover = { glyph: e[0], name: e[1] })} title=":{e[1]}:">{e[0]}</button>
            {/each}
          </div>
          {#if !shownCustom.length && !shownUnicode.length}<p class="muted">{t('emoji_picker_nothing_found_q', { q: query.text })}</p>{/if}
        {:else}
          {#each servers as x (x.g.id)}
            <section data-section="g-{x.g.id}">
              <h4>{x.g.name}</h4>
              <div class="grid">
                {#each x.emojis as e (e.id)}
                  <button onclick={() => onpick(`<:${e.name}:${e.file}>`, 'custom', e)} onmouseenter={() => (hover = { file: e.file, name: e.name })} title=":{e.name}:"><img src={asset(e.file)} alt=":{e.name}:" /></button>
                {/each}
              </div>
            </section>
          {/each}
          {#each categories as [id, list] (id)}
            <section data-section={id}>
              <h4>{t(`emoji_picker_cat_${id}`)}</h4>
              <div class="grid">
                {#each list as e (e[0])}
                  <button onclick={() => onpick(e[0], 'unicode')} onmouseenter={() => (hover = { glyph: e[0], name: e[1] })} title=":{e[1]}:">{e[0]}</button>
                {/each}
              </div>
            </section>
          {/each}
        {/if}
      {:else}
        <div class="grid big">
          {#each shownStickers as e (e.id)}
            <button onclick={() => onpick(`<sticker:${e.name}:${e.file}>`, 'sticker', e)} title={e.name}><img src={asset(e.file)} alt={e.name} /></button>
          {:else}
            <p class="muted">{q ? t('emoji_picker_nothing_found_q', { q: query.text }) : t('emoji_picker_no_stickers_yet_upload_some')}</p>
          {/each}
        </div>
      {/if}
    </div>
  </div>
  {#if tab === 'emoji'}
    <footer class="preview">
      {#if hover}
        <span class="big">{#if hover.file}<img src={asset(hover.file)} alt="" />{:else}{hover.glyph}{/if}</span>
        <span class="name">:{hover.name}:</span>
      {/if}
    </footer>
  {/if}
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 30;
  }
  .picker {
    animation: pop-in 0.12s ease-out;
    position: absolute;
    right: 12px;
    bottom: 74px;
    z-index: 31;
    width: min(400px, calc(100% - 24px));
    display: flex;
    flex-direction: column;
    border-radius: var(--radius) !important;
    border: var(--panel-border) !important;
    box-shadow: var(--panel-shadow) !important;
    background: var(--panel-2);
    overflow: hidden;
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
    margin: 10px 10px 8px;
    width: calc(100% - 20px);
  }
  .layout {
    display: flex;
    min-height: 0;
    border-top: 1px solid var(--line);
  }
  .rail {
    flex: none;
    width: 42px;
    padding: 6px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    background: var(--panel);
    max-height: 320px;
    overflow-y: auto;
    scrollbar-width: none;
  }
  .rail.dim {
    opacity: 0.5;
  }
  .rail button {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--muted);
  }
  .rail button:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .rail button.on {
    color: var(--ink);
    background: var(--panel-2);
  }
  .sep {
    width: 20px;
    height: 1px;
    flex: none;
    background: var(--line);
    margin: 4px 0;
  }
  .body {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 320px;
    padding: 0 10px 10px;
  }
  h4 {
    position: sticky;
    top: 0;
    z-index: 1;
    margin: 0;
    padding: 10px 2px 6px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
    background: var(--panel-2);
  }
  .body > .grid:first-child {
    margin-top: 10px;
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
  .preview {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    padding: 0 12px;
    border-top: 1px solid var(--line);
    background: var(--panel);
  }
  .preview .big {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    font-size: 1.6rem;
    font-family: system-ui, 'Noto Color Emoji', sans-serif;
  }
  .preview .big img {
    width: 100%;
    height: 100%;
  }
  .preview .name {
    font-weight: 600;
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
