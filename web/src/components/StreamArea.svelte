<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { setStreamVolume, voice, watchStream } from '../lib/voice.svelte';
  import Icon from './Icon.svelte';

  let { compact = false }: { compact?: boolean } = $props();

  function show(node: HTMLVideoElement, track: any) {
    track.attach(node);
    return { destroy: () => track.detach(node) };
  }

  function viewers(userId: string) {
    return (voice.watchers[userId] ?? []).map((id) => voice.names[id] ?? '?');
  }
</script>

{#if voice.streams.length}
  <div class="streams" class:compact class:single={voice.streams.length === 1}>
    {#each voice.streams as s (s.userId)}
      {@const tile = voice.tiles.find((x) => x.userId === s.userId && x.kind === 'screen')}
      {@const list = viewers(s.userId)}
      <figure>
        {#if s.watching && tile}
          <video use:show={tile.track} autoplay playsinline muted={s.local}></video>
        {:else}
          <div class="closed">
            <Icon name="broadcast" size={28} />
            <strong>{t('stream_area_name_sharing_their_screen', { name: s.name })}</strong>
            <button class="btn primary small" onclick={() => watchStream(s.userId, true)}><Icon name="screen" size={15} />{t('stream_area_watch_stream')}</button>
          </div>
        {/if}
        <figcaption>
          <div class="line">
            <span class="ellipsis">{s.local ? t('stream_area_stream') : s.name}</span>
            <span class="count" title={list.join(', ')}><Icon name="users" size={13} />{list.length}</span>
            {#if s.watching && !s.local && s.audio}
              {@const level = voice.streamVolume[s.userId] ?? 1}
              <span class="volume">
                <button class="mute" class:off={level === 0} onclick={() => setStreamVolume(s.userId, level === 0 ? 1 : 0)} aria-label={level === 0 ? t('stream_area_unmute_stream') : t('stream_area_mute_stream')} title={level === 0 ? t('stream_area_unmute_stream') : t('stream_area_mute_stream')}><Icon name="volume" size={14} /></button>
                <input type="range" min="0" max="1" step="0.05" value={level} oninput={(e) => setStreamVolume(s.userId, Number(e.currentTarget.value))} aria-label={t('stream_area_stream_volume')} />
              </span>
            {:else if s.local && !s.audio}
              <span class="noaudio">{t('stream_area_without_sound')}</span>
            {/if}
            {#if s.watching && !s.local}<button onclick={() => watchStream(s.userId, false)}>{t('stream_area_stop_watching')}</button>{/if}
          </div>
          {#if list.length}<p class="who ellipsis">{t('stream_area_watching_names', { names: list.join(', ') })}</p>{/if}
        </figcaption>
      </figure>
    {/each}
  </div>
{/if}

<style>
  .streams {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 12px;
  }
  .streams.single {
    grid-template-columns: 1fr;
  }
  .streams.compact {
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    padding: 10px 14px 0;
    gap: 8px;
  }
  figure {
    margin: 0;
    border-radius: var(--radius-s);
    overflow: hidden;
    background: #000;
    color: #fff;
    display: flex;
    flex-direction: column;
  }
  video {
    display: block;
    width: 100%;
    max-height: 62vh;
    object-fit: contain;
  }
  .compact video {
    max-height: 42vh;
  }
  .closed {
    min-height: 150px;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 8px;
    padding: 18px;
    text-align: center;
    background: var(--panel-2);
    color: var(--ink);
  }
  figcaption {
    padding: 5px 10px;
    font-size: 0.85em;
    background: rgba(0, 0, 0, 0.75);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .line .ellipsis {
    flex: 1;
    font-weight: 600;
  }
  .count {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .line button {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .who {
    font-size: 0.92em;
    opacity: 0.8;
  }
  .volume {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .volume input {
    width: 80px;
    accent-color: var(--accent);
  }
  .mute {
    display: inline-flex;
    padding: 2px;
  }
  .mute.off {
    opacity: 0.45;
    text-decoration: line-through;
  }
  .noaudio {
    font-size: 0.8em;
    opacity: 0.7;
  }
</style>
