<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { asset } from '../lib/api';
  import { app, guild, type Emoji } from '../lib/store.svelte';

  let { stickers = false, onpick, onclose }: { stickers?: boolean; onpick: (value: string, kind: 'unicode' | 'custom' | 'sticker', emoji?: Emoji) => void; onclose: () => void } = $props();

  const UNICODE = '😀 😂 🤣 😊 😍 😘 😎 🤔 😅 😇 🙃 😉 😢 😭 😡 🤯 😱 🥳 😴 🤝 👍 👎 👏 🙏 💪 👀 🔥 ✨ 🎉 ❤️ 💔 💯 ✅ ❌ ⚠️ ❓ ⭐ 🌙 ☀️ 🍕 🍺 ☕ 🎮 🎲 🏆 ⚔️ 🛡️ 🐉 👑 💎 🗝️ 📜 🧪 🚀 💤 🎵'.split(' ');

  let tab = $state<'emoji' | 'sticker'>('emoji');
  const sorted = $derived([...app.guilds].sort((a, b) => Number(b.id === guild()?.id) - Number(a.id === guild()?.id)));
  const custom = $derived(sorted.flatMap((g) => g.emojis.filter((e) => e.kind === 'emoji')));
  const stickerList = $derived(sorted.flatMap((g) => g.emojis.filter((e) => e.kind === 'sticker')));
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
  <div class="scroll body">
    {#if tab === 'emoji'}
      {#if custom.length}
        <div class="grid">
          {#each custom as e (e.id)}
            <button onclick={() => onpick(`<:${e.name}:${e.file}>`, 'custom', e)} title=":{e.name}:"><img src={asset(e.file)} alt=":{e.name}:" /></button>
          {/each}
        </div>
        <hr />
      {/if}
      <div class="grid">
        {#each UNICODE as e}
          <button onclick={() => onpick(e, 'unicode')}>{e}</button>
        {/each}
      </div>
    {:else}
      <div class="grid big">
        {#each stickerList as e (e.id)}
          <button onclick={() => onpick(`<sticker:${e.name}:${e.file}>`, 'sticker', e)} title={e.name}><img src={asset(e.file)} alt={e.name} /></button>
        {:else}
          <p class="muted">{t('Noch keine Sticker. In den Servereinstellungen unter „Emojis“ lassen sich welche hochladen.')}</p>
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
