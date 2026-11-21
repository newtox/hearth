type Layer = { id: number; prio: number; active: () => boolean; close: () => void };

const state = $state<{ layers: Layer[]; pushed: boolean }>({ layers: [], pushed: false });
let seq = 0;
let skip = 0;

function top() {
  let best: Layer | null = null;
  for (const l of state.layers) {
    if (!l.active()) continue;
    if (!best || l.prio > best.prio || (l.prio === best.prio && l.id > best.id)) best = l;
  }
  return best;
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
    const open = !!top();
    if (open && !state.pushed) {
      history.pushState({ hearth: 'back' }, '');
      state.pushed = true;
    } else if (!open && state.pushed) {
      state.pushed = false;
      skip++;
      history.back();
    }
  });
});

addEventListener('popstate', () => {
  if (skip) {
    skip--;
    return;
  }
  state.pushed = false;
  top()?.close();
});
