import { base, isApp, isElectron } from './api';

export function openExternal(url: string) {
  if (isElectron) (window as any).hearthDesktop.openExternal(url);
  else if (isApp) (window as any).__TAURI_INTERNALS__.invoke('plugin:opener|open_url', { url }).catch(() => window.open(url, '_blank', 'noopener'));
  else window.open(url, '_blank', 'noopener');
}

export function webAddress() {
  return base() || location.origin;
}

if (isApp) {
  document.addEventListener(
    'click',
    (e) => {
      const a = (e.target as HTMLElement | null)?.closest('a[href]') as HTMLAnchorElement | null;
      if (!a || a.hasAttribute('download')) return;
      const href = a.href;
      if (!/^https?:/i.test(href) || (a.target !== '_blank' && new URL(href).origin === location.origin)) return;
      e.preventDefault();
      openExternal(href);
    },
    true,
  );
}
