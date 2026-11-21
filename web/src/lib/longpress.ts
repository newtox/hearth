export {};

const HOLD = 450;

let timer: ReturnType<typeof setTimeout> | undefined;
let start: { x: number; y: number; id: number; target: HTMLElement } | null = null;
let fired = false;
let native = false;

function reset() {
  clearTimeout(timer);
  timer = undefined;
  start = null;
}

function skip(el: HTMLElement) {
  return !!el.closest('input, textarea, select, [contenteditable="true"], [data-sort-live], .menu');
}

addEventListener(
  'pointerdown',
  (e) => {
    reset();
    fired = false;
    native = false;
    if (e.pointerType === 'mouse') return;
    const target = e.target as HTMLElement;
    if (skip(target)) return;
    start = { x: e.clientX, y: e.clientY, id: e.pointerId, target };
    timer = setTimeout(() => {
      if (!start || native) return;
      fired = true;
      navigator.vibrate?.(12);
      start.target.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: start.x, clientY: start.y }));
      reset();
    }, HOLD);
  },
  { capture: true },
);

addEventListener(
  'pointermove',
  (e) => {
    if (start && e.pointerId === start.id && Math.hypot(e.clientX - start.x, e.clientY - start.y) > 10) reset();
  },
  { capture: true },
);

addEventListener('pointerup', reset, { capture: true });
addEventListener('pointercancel', reset, { capture: true });

addEventListener(
  'contextmenu',
  (e) => {
    if (e.isTrusted && start) {
      native = true;
      fired = true;
      reset();
    }
  },
  { capture: true },
);

addEventListener(
  'click',
  (e) => {
    if (!fired) return;
    fired = false;
    e.preventDefault();
    e.stopPropagation();
  },
  { capture: true },
);
