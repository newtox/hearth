export type Place = 'before' | 'after' | 'inside';

export interface SortOptions {
  id: string;
  group: string;
  label: string;
  enabled?: boolean;
  place: (dragId: string, targetId: string, ratio: number) => Place | null;
  drop: (dragId: string, targetId: string, place: Place) => void;
}

const MARKS = ['drop-before', 'drop-after', 'drop-inside'];

export function sortable(node: HTMLElement, initial: SortOptions) {
  let opts = initial;
  let start: { x: number; y: number; id: number; touch: boolean } | null = null;
  let dragging = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let ghost: HTMLElement | null = null;
  let target: { el: HTMLElement; id: string; place: Place } | null = null;

  node.dataset.sortGroup = opts.group;
  node.dataset.sortId = opts.id;

  function clearMark() {
    target?.el.classList.remove(...MARKS);
    target = null;
  }

  function begin() {
    dragging = true;
    node.classList.add('dragging');
    document.body.classList.add('sorting');
    ghost = document.createElement('div');
    ghost.className = 'drag-ghost';
    ghost.textContent = opts.label;
    document.body.append(ghost);
  }

  function scrollNear(y: number) {
    const box = node.closest('.scroll') as HTMLElement | null;
    if (!box) return;
    const r = box.getBoundingClientRect();
    if (y < r.top + 36) box.scrollTop -= 10;
    else if (y > r.bottom - 36) box.scrollTop += 10;
  }

  function move(e: PointerEvent) {
    if (!start || e.pointerId !== start.id) return;
    const dist = Math.hypot(e.clientX - start.x, e.clientY - start.y);
    if (!dragging) {
      if (start.touch) {
        if (dist > 8) cancel();
        return;
      }
      if (dist < 5) return;
      begin();
    }
    e.preventDefault();
    ghost!.style.transform = `translate(${e.clientX + 12}px, ${e.clientY + 8}px)`;
    scrollNear(e.clientY);
    const el = (document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null)?.closest(`[data-sort-group="${opts.group}"]`) as HTMLElement | null;
    const id = el?.dataset.sortId;
    let place: Place | null = null;
    if (el && id) {
      const r = el.getBoundingClientRect();
      place = opts.place(opts.id, id, (e.clientY - r.top) / r.height);
    }
    if (target && (target.el !== el || target.place !== place)) clearMark();
    if (el && id && place) {
      target = { el, id, place };
      el.classList.add(`drop-${place}`);
    }
  }

  function stopTouch(e: TouchEvent) {
    if (dragging) e.preventDefault();
  }

  function finish() {
    if (timer) clearTimeout(timer);
    timer = null;
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    window.removeEventListener('pointercancel', cancel);
    window.removeEventListener('touchmove', stopTouch);
    ghost?.remove();
    ghost = null;
    node.classList.remove('dragging');
    document.body.classList.remove('sorting');
    start = null;
  }

  function cancel() {
    clearMark();
    dragging = false;
    finish();
  }

  function up(e: PointerEvent) {
    if (!start || e.pointerId !== start.id) return;
    const done = dragging ? target : null;
    const was = dragging;
    clearMark();
    finish();
    dragging = false;
    if (was) {
      const swallow = (ev: Event) => {
        ev.stopPropagation();
        ev.preventDefault();
      };
      node.addEventListener('click', swallow, { capture: true, once: true });
      setTimeout(() => node.removeEventListener('click', swallow, { capture: true }), 0);
    }
    if (done) opts.drop(opts.id, done.id, done.place);
  }

  function down(e: PointerEvent) {
    if (opts.enabled === false || e.button !== 0 || start) return;
    if ((e.target as HTMLElement).closest('input, textarea, select, [data-no-drag]')) return;
    start = { x: e.clientX, y: e.clientY, id: e.pointerId, touch: e.pointerType !== 'mouse' };
    window.addEventListener('pointermove', move, { passive: false });
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', cancel);
    window.addEventListener('touchmove', stopTouch, { passive: false });
    if (start.touch) timer = setTimeout(() => (navigator.vibrate?.(15), begin()), 380);
  }

  function blockMenu(e: Event) {
    if (dragging) e.preventDefault();
  }

  node.addEventListener('pointerdown', down);
  node.addEventListener('contextmenu', blockMenu);
  return {
    update(next: SortOptions) {
      opts = next;
      node.dataset.sortGroup = opts.group;
      node.dataset.sortId = opts.id;
    },
    destroy() {
      cancel();
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('contextmenu', blockMenu);
    },
  };
}
