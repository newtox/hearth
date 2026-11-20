import { serverError, t } from './i18n.svelte';

export const isTauri = '__TAURI_INTERNALS__' in window;
export const isElectron = 'hearthDesktop' in window;
export const isApp = isTauri || isElectron;
const client = !isApp ? 'web' : /android/i.test(navigator.userAgent) ? 'android' : 'desktop';

export function base(): string {
  return (localStorage.getItem('hearth_server') ?? import.meta.env.VITE_SERVER_URL ?? '').replace(/\/+$/, '');
}

export function setServer(url: string) {
  const clean = url.trim().replace(/\/+$/, '');
  if (clean) localStorage.setItem('hearth_server', /^https?:\/\//.test(clean) ? clean : `https://${clean}`);
  else localStorage.removeItem('hearth_server');
}

let token: string | null = localStorage.getItem('hearth_token');

export function getToken() {
  return token;
}

export function setToken(t: string | null) {
  token = t;
  if (t) localStorage.setItem('hearth_token', t);
  else localStorage.removeItem('hearth_token');
}

export function thumb(name: string | null | undefined, size: number) {
  return name ? (name.includes('/') ? asset(name) : `${base()}/thumbs/${size}/${name}`) : null;
}

export function asset(name: string | null | undefined) {
  return name ? `${base()}/uploads/${name}` : null;
}

export function gatewayUrl() {
  if (base()) return base().replace(/^http/, 'ws') + '/gateway';
  return `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/gateway`;
}

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function send(method: string, path: string, init: RequestInit) {
  const headers = new Headers(init.headers);
  if (token) headers.set('Authorization', `Bearer ${token}`);
  headers.set('X-Hearth-Client', client);
  let res: Response;
  try {
    res = await fetch(`${base()}/api${path}`, { ...init, method, headers });
  } catch {
    throw new ApiError(0, t('api_server_unreachable'));
  }
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(res.status, data?.error ? serverError(data.error) : `${t('common_error')} ${res.status}`);
  return data;
}

export function api<T = any>(method: string, path: string, body?: unknown): Promise<T> {
  return send(method, path, body === undefined ? {} : { body: JSON.stringify(body), headers: { 'Content-Type': 'application/json' } });
}

export function upload<T = any>(path: string, file: File): Promise<T> {
  const form = new FormData();
  form.append('file', file);
  return send('POST', path, { body: form });
}
