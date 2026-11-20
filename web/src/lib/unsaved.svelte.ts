import { ask } from './dialog.svelte';
import { t } from './i18n.svelte';

export interface Guard {
  dirty: () => boolean;
  reset: () => void;
}

const guards = new Set<Guard>();

export function guard(g: Guard) {
  guards.add(g);
  return () => {
    guards.delete(g);
  };
}

export async function leave() {
  const open = [...guards].filter((g) => g.dirty());
  if (!open.length) return true;
  const ok = await ask({ title: t('unsaved_title'), text: t('unsaved_text'), confirm: t('unsaved_discard'), danger: true });
  if (ok) open.forEach((g) => g.reset());
  return ok;
}
