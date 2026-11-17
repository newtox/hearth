<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { untrack } from 'svelte';
  import { setStreamVolume, voice, watchStream } from '../lib/voice.svelte';
  import Icon from './Icon.svelte';

  let { compact = false }: { compact?: boolean } = $props();

  function show(node: HTMLVideoElement, track: any) {
    track.attach(node);
    return { destroy: () => track.detach(node) };
  }

  const key = untrack(() => compact) ? 'hearth_stream_h_call' : 'hearth_stream_h';
  let height = $state(Number(localStorage.getItem(key)) || 0);

  function resize(e: PointerEvent) {
    const fig = (e.currentTarget as HTMLElement).parentElement?.querySelector('video, .closed') as HTMLElement | null;
    const start = { y: e.clientY, h: fig?.getBoundingClientRect().height ?? 300 };
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      height = Math.round(Math.min(window.innerHeight - 140, Math.max(120, start.h + ev.clientY - start.y)));
    };
    const up = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
      try {
        localStorage.setItem(key, String(height));
      } catch {}
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  }

  function resetSize() {
    height = 0;
    try {
      localStorage.removeItem(key);
    } catch {}
  }

  function fullscreen(e: MouseEvent) {
    const fig = (e.currentTarget as HTMLElement).closest('figure');
    if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    else fig?.requestFullscreen?.().catch(() => {});
  }

  function viewers(userId: string) {
    return (voice.watchers[userId] ?? []).map((id) => voice.names[id] ?? '?');
  }
</script>

{#if voice.streams.length}
  <div class="streams" class:compact class:single={voice.streams.length === 1} style:--stream-h={height ? `${height}px` : null}>
    {#each voice.streams as s (s.userId)}
      {@const tile = voice.tiles.find((x) => x.userId === s.userId && x.kind === 'screen')}
      {@const list = viewers(s.userId)}
      <figure>
        {#if s.watching && tile}
          <video use:show={tile.track} autoplay playsinline muted={s.local} ondblclick={fullscreen}></video>
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
            {#if s.watching && tile}<button class="full" onclick={fullscreen} aria-label={t('stream_area_fullscreen')} title={t('stream_area_fullscreen')}><Icon name="expand" size={14} /></button>{/if}
          </div>
          {#if list.length}<p class="who ellipsis">{t('stream_area_watching_names', { names: list.join(', ') })}</p>{/if}
        </figcaption>
      </figure>
    {/each}
    <div class="grip" role="separator" aria-orientation="horizontal" aria-label={t('stream_area_resize')} title={t('stream_area_resize')} onpointerdown={resize} ondblclick={resetSize}></div>
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
    max-height: var(--stream-h, 62vh);
    height: var(--stream-h, auto);
    object-fit: contain;
  }
  .compact video {
    max-height: var(--stream-h, 42vh);
  }
  figure:fullscreen video {
    max-height: none;
    height: calc(100% - 34px);
  }
  figure:fullscreen {
    justify-content: center;
  }
  .full {
    display: inline-flex;
    padding: 2px;
    text-decoration: none !important;
  }
  .grip {
    grid-column: 1 / -1;
    height: 10px;
    margin-top: -6px;
    cursor: ns-resize;
    touch-action: none;
    position: relative;
  }
  .grip::after {
    content: '';
    position: absolute;
    left: 50%;
    top: 3px;
    width: 48px;
    height: 4px;
    margin-left: -24px;
    border-radius: 2px;
    background: var(--line);
  }
  .grip:hover::after {
    background: var(--accent);
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
