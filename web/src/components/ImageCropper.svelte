<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../lib/i18n.svelte';
  import { crop } from '../lib/crop.svelte';
  import Modal from './Modal.svelte';

  const c = $derived(crop.current);
  const banner = $derived(c?.shape === 'banner');
  let avail = $state(400);
  const W = $derived(Math.min(banner ? 400 : 280, avail || 280));
  const H = $derived(banner ? Math.round(W * 0.4) : W);
  const OUT = $derived(banner ? { w: 1200, h: 480 } : { w: 512, h: 512 });

  let img = $state<HTMLImageElement>();
  let natural = $state({ w: 0, h: 0 });
  let zoom = $state(1);
  let pos = $state({ x: 0, y: 0 });
  let drag: { x: number; y: number; px: number; py: number } | null = null;
  let busy = $state(false);

  const base = $derived(natural.w ? Math.max(W / natural.w, H / natural.h) : 1);
  const scale = $derived(base * zoom);
  const size = $derived({ w: natural.w * scale, h: natural.h * scale });

  function clamp(x: number, y: number) {
    return { x: Math.min(0, Math.max(W - size.w, x)), y: Math.min(0, Math.max(H - size.h, y)) };
  }

  function loaded() {
    if (!img) return;
    natural = { w: img.naturalWidth, h: img.naturalHeight };
    zoom = 1;
    pos = clamp((W - natural.w * base) / 2, (H - natural.h * base) / 2);
  }

  $effect(() => {
    W;
    H;
    untrack(() => natural.w && loaded());
  });

  function setZoom(next: number, cx = W / 2, cy = H / 2) {
    const z = Math.min(4, Math.max(1, next));
    const ix = (cx - pos.x) / scale;
    const iy = (cy - pos.y) / scale;
    zoom = z;
    pos = clamp(cx - ix * base * z, cy - iy * base * z);
  }

  function down(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, y: e.clientY, px: pos.x, py: pos.y };
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    pos = clamp(drag.px + e.clientX - drag.x, drag.py + e.clientY - drag.y);
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setZoom(zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1), e.clientX - r.left, e.clientY - r.top);
  }

  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 20 : 5;
    const moves: Record<string, [number, number]> = { ArrowLeft: [step, 0], ArrowRight: [-step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] };
    if (moves[e.key]) {
      e.preventDefault();
      pos = clamp(pos.x + moves[e.key][0], pos.y + moves[e.key][1]);
    } else if (e.key === '+' || e.key === '=') setZoom(zoom * 1.1);
    else if (e.key === '-') setZoom(zoom / 1.1);
  }

  async function save() {
    if (!c || !img) return;
    busy = true;
    const canvas = document.createElement('canvas');
    canvas.width = OUT.w;
    canvas.height = OUT.h;
    const ctx = canvas.getContext('2d')!;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(img, -pos.x / scale, -pos.y / scale, W / scale, H / scale, 0, 0, OUT.w, OUT.h);
    const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/webp', 0.92));
    busy = false;
    const name = c.file.name.replace(/\.[^.]+$/, '') + '.webp';
    c.resolve(blob ? new File([blob], name, { type: 'image/webp' }) : null);
  }
</script>

{#if c}
  <Modal title={t('crop_title')} onclose={() => c.resolve(null)}>
    <div class="stack" bind:clientWidth={avail}>
      <div
        class="stage"
        class:circle={c.shape === 'circle'}
        style:width="{W}px"
        style:height="{H}px"
        onpointerdown={down}
        onpointermove={move}
        onpointerup={() => (drag = null)}
        onpointercancel={() => (drag = null)}
        onwheel={wheel}
        onkeydown={key}
        role="slider"
        tabindex="0"
        aria-label={t('crop_hint')}
        aria-valuemin={1}
        aria-valuemax={4}
        aria-valuenow={Math.round(zoom * 100) / 100}
      >
        <img bind:this={img} src={c.url} alt="" onload={loaded} draggable="false" style:width="{size.w}px" style:height="{size.h}px" style:transform="translate({pos.x}px, {pos.y}px)" />
        <div class="mask"></div>
      </div>
      <p class="muted small">{t('crop_hint')}</p>
      <label class="field zoom">
        <span>{t('crop_zoom')}</span>
        <input type="range" min="1" max="4" step="0.01" value={zoom} oninput={(e) => setZoom(Number((e.target as HTMLInputElement).value))} />
      </label>
      <div class="row end">
        <button type="button" class="btn" onclick={() => c.resolve(null)}>{t('common_cancel')}</button>
        <button type="button" class="btn primary" onclick={save} disabled={busy || !natural.w}>{t('crop_apply')}</button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .stage {
    position: relative;
    max-width: 100%;
    margin: 0 auto;
    overflow: hidden;
    border-radius: var(--radius-s);
    background: repeating-conic-gradient(var(--panel-2) 0 25%, var(--panel) 0 50%) 0 0 / 16px 16px;
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .stage:active {
    cursor: grabbing;
  }
  .stage:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  img {
    position: absolute;
    top: 0;
    left: 0;
    max-width: none;
    transform-origin: 0 0;
    pointer-events: none;
  }
  .mask {
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.85);
  }
  .circle .mask {
    border-radius: 50%;
    box-shadow:
      0 0 0 999px rgba(0, 0, 0, 0.55),
      inset 0 0 0 2px rgba(255, 255, 255, 0.85);
  }
  .zoom input {
    width: 100%;
  }
  .end {
    justify-content: flex-end;
  }
  .small {
    font-size: 0.85em;
    margin: 0;
    text-align: center;
  }
</style>
