import { asset } from './api';

const frames = $state<Record<string, string>>({});
const pending = new Set<string>();

export function isAnimated(url: string | null | undefined) {
  return !!url && /\.(gif|webp)(\?|$)/i.test(url);
}

function render(img: HTMLImageElement, size: number) {
  let source: CanvasImageSource = img;
  let w = img.naturalWidth;
  let h = img.naturalHeight;
  const scale = Math.min(1, size / Math.min(w, h));
  const tw = Math.max(1, Math.round(w * scale));
  const th = Math.max(1, Math.round(h * scale));
  while (w / 2 >= tw && h / 2 >= th) {
    const step = document.createElement('canvas');
    step.width = Math.round(w / 2);
    step.height = Math.round(h / 2);
    const ctx = step.getContext('2d')!;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(source, 0, 0, step.width, step.height);
    source = step;
    w = step.width;
    h = step.height;
  }
  const out = document.createElement('canvas');
  out.width = tw;
  out.height = th;
  const ctx = out.getContext('2d')!;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(source, 0, 0, tw, th);
  return out.toDataURL('image/png');
}

export function frame(url: string, size: number) {
  const key = `${size}|${url}`;
  if (frames[key]) return frames[key];
  if (!pending.has(key)) {
    pending.add(key);
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      try {
        frames[key] = Math.min(img.naturalWidth, img.naturalHeight) <= size && !isAnimated(url) ? url : render(img, size);
      } catch {
        frames[key] = url;
      }
    };
    img.onerror = () => (frames[key] = url);
    img.src = url;
  }
  return null;
}

export function sizeFor(px: number) {
  const want = px * Math.max(1, typeof devicePixelRatio === 'number' ? devicePixelRatio : 1);
  return [32, 64, 128, 256].find((s) => s >= want) ?? 256;
}

export function still(url: string) {
  return isAnimated(url) ? frame(url, 1024) : url;
}

export function backdrop(file: string | null | undefined, moving: boolean) {
  const url = asset(file);
  if (!url) return null;
  const shown = moving || !isAnimated(url) ? url : still(url);
  return shown ? `url(${shown})` : null;
}

export function hoverHost(node: HTMLElement, set: (on: boolean) => void) {
  const host = (node.parentElement?.closest('button, a, article, li, [role="button"], [data-hover]') as HTMLElement | null) ?? node;
  const enter = () => set(true);
  const leave = () => set(false);
  host.addEventListener('pointerenter', enter);
  host.addEventListener('pointerleave', leave);
  host.addEventListener('focusin', enter);
  host.addEventListener('focusout', leave);
  return {
    destroy() {
      host.removeEventListener('pointerenter', enter);
      host.removeEventListener('pointerleave', leave);
      host.removeEventListener('focusin', enter);
      host.removeEventListener('focusout', leave);
    },
  };
}
