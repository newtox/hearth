import { isElectron, isTauri } from './api';
import { openExternal } from './external';
import { t } from './i18n.svelte';
import { toast } from './store.svelte';

declare const __BUILD_ID__: string;

type Mode = 'reload' | 'install' | 'download';

export const update = $state({
  current: '',
  latest: null as string | null,
  mode: null as Mode | null,
  url: '',
  busy: false,
  progress: null as number | null,
  hidden: false,
});

const desktop = isElectron ? (window as any).hearthDesktop : null;
const tauri = isTauri ? (window as any).__TAURI_INTERNALS__ : null;
const android = isTauri && /android/i.test(navigator.userAgent);
const repoUrl = ((import.meta.env.VITE_REPO_URL as string | undefined) ?? 'https://github.com/newtox/hearth').replace(/\/+$/, '');
const repo = repoUrl.replace(/^https?:\/\/github\.com\//, '');

function newer(a: string, b: string) {
  const parts = (v: string) => v.replace(/^v/, '').split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
  }
  return false;
}

async function ownVersion(): Promise<string> {
  if (desktop) return desktop.version().catch(() => '');
  if (tauri) return tauri.invoke('plugin:app|version').catch(() => '');
  return '';
}

async function latestRelease() {
  const res = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, { headers: { Accept: 'application/vnd.github+json' } });
  if (!res.ok) throw new Error(String(res.status));
  const data = await res.json();
  const assets: { name: string; browser_download_url: string }[] = data.assets ?? [];
  const asset = (re: RegExp) => assets.find((a) => re.test(a.name))?.browser_download_url;
  const url = (android ? asset(/\.apk$/i) : null) ?? data.html_url ?? `${repoUrl}/releases/latest`;
  return { version: String(data.tag_name ?? '').replace(/^v/, ''), url };
}

function found(version: string | null, mode: Mode | null, url = '') {
  const fresh = version !== update.latest || mode !== update.mode;
  update.latest = version;
  update.mode = version ? mode : null;
  update.url = url;
  if (fresh) update.hidden = false;
}

async function checkWeb() {
  const res = await fetch(`${import.meta.env.BASE_URL}version.json`, { cache: 'no-store' });
  if (!res.ok) return false;
  const { build } = await res.json();
  const available = !!build && build !== __BUILD_ID__;
  found(available ? 'web' : null, 'reload');
  return available;
}

async function checkRelease() {
  const release = await latestRelease();
  const available = !!update.current && !!release.version && newer(release.version, update.current);
  found(available ? release.version : null, 'download', release.url);
  return available;
}

export async function checkForUpdate(manual = false) {
  if (update.busy) return;
  try {
    update.current ||= await ownVersion();
    let available = false;
    if (!isElectron && !isTauri) available = await checkWeb();
    else if (desktop) {
      const res = await desktop.updateCheck().catch(() => ({ supported: false }));
      if (res.supported) {
        found(res.version ?? null, 'install');
        available = !!res.version;
      } else available = await checkRelease();
    } else if (tauri && !android) {
      try {
        const version: string | null = await tauri.invoke('update_check');
        found(version, 'install');
        available = !!version;
      } catch {
        available = await checkRelease();
      }
    } else available = await checkRelease();
    if (manual && !available) toast(t('update_up_to_date'));
  } catch {
    if (manual) toast(t('update_check_failed'));
  }
}

async function onProgress(fn: (percent: number) => void) {
  if (desktop) desktop.onUpdateProgress(fn);
  else if (tauri) {
    const handler = tauri.transformCallback((e: { payload: number }) => fn(e.payload));
    await tauri.invoke('plugin:event|listen', { event: 'update-progress', target: { kind: 'Any' }, handler }).catch(() => {});
  }
}

let listening = false;

export async function installUpdate() {
  if (!update.mode || update.busy) return;
  if (update.mode === 'reload') return location.reload();
  if (update.mode === 'download') return openExternal(update.url || `${repoUrl}/releases/latest`);
  update.busy = true;
  update.progress = 0;
  if (!listening) {
    listening = true;
    await onProgress((p) => (update.progress = Math.max(0, Math.min(100, Math.round(p)))));
  }
  const ok = desktop ? await desktop.updateInstall().catch(() => false) : await tauri.invoke('update_install').then(() => true).catch(() => false);
  if (!ok) {
    update.busy = false;
    update.progress = null;
    toast(t('update_install_failed'));
  }
}

let started = false;

export function watchUpdates() {
  if (started) return;
  started = true;
  setTimeout(() => checkForUpdate(), 8000);
  setInterval(() => checkForUpdate(), (isElectron || isTauri ? 6 * 60 : 30) * 60 * 1000);
  let lastFocus = Date.now();
  document.addEventListener('visibilitychange', () => {
    if (document.hidden || Date.now() - lastFocus < 10 * 60 * 1000) return;
    lastFocus = Date.now();
    checkForUpdate();
  });
}
