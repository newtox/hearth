type Layer = { id: number; prio: number; active: () => boolean; close: () => void };

const state = $state<{ layers: Layer[]; tick: number }>({ layers: [], tick: 0 });
let seq = 0;
let leaving = false;
let timer: ReturnType<typeof setTimeout> | undefined;

function top() {
  let best: Layer | null = null;
  for (const l of state.layers) {
    if (!l.active()) continue;
    if (!best || l.prio > best.prio || (l.prio === best.prio && l.id > best.id)) best = l;
  }
  return best;
}

const onSentinel = () => history.state?.hearth === 'back';

function sync() {
  clearTimeout(timer);
  timer = undefined;
  if (leaving) return;
  const open = !!top();
  if (open && !onSentinel()) {
    history.pushState({ hearth: 'back' }, '');
  } else if (!open && onSentinel()) {
    timer = setTimeout(() => {
      timer = undefined;
      if (leaving || top() || !onSentinel()) return;
      leaving = true;
      history.back();
    }, 120);
  }
}

export function onBack(close: () => void, prio = 10, active: () => boolean = () => true) {
  const layer = { id: ++seq, prio, active, close };
  state.layers.push(layer);
  return () => {
    state.layers = state.layers.filter((l) => l.id !== layer.id);
  };
}

export function backLayer(node: HTMLElement, close: () => void) {
  let current = close;
  const off = onBack(() => current());
  return {
    update(next: () => void) {
      current = next;
    },
    destroy: off,
  };
}

$effect.root(() => {
  $effect(() => {
    state.tick;
    top();
    sync();
  });
});

addEventListener('popstate', () => {
  if (leaving) {
    leaving = false;
    sync();
    return;
  }
  top()?.close();
  queueMicrotask(() => {
    state.tick++;
    sync();
  });
});
