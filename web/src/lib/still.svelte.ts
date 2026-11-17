import { asset } from './api';

const frames = $state<Record<string, string>>({});
const pending = new Set<string>();

export function isAnimated(url: string | null | undefined) {
  return !!url && /\.(gif|webp)(\?|$)/i.test(url);
}

export function still(url: string) {
  if (!isAnimated(url)) return url;
  if (frames[url]) return frames[url];
  if (!pending.has(url)) {
    pending.add(url);
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      try {
        const scale = Math.min(1, 960 / Math.max(img.naturalWidth, img.naturalHeight));
        const canvas = document.createElement('canvas');
        canvas.width = Math.max(1, Math.round(img.naturalWidth * scale));
        canvas.height = Math.max(1, Math.round(img.naturalHeight * scale));
        canvas.getContext('2d')!.drawImage(img, 0, 0, canvas.width, canvas.height);
        frames[url] = canvas.toDataURL('image/png');
      } catch {
        frames[url] = url;
      }
    };
    img.onerror = () => (frames[url] = url);
    img.src = url;
  }
  return null;
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

export function backdrop(file: string | null | undefined, moving: boolean) {
  const url = asset(file);
  if (!url) return null;
  const shown = moving || !isAnimated(url) ? url : still(url);
  return shown ? `url(${shown})` : null;
}
