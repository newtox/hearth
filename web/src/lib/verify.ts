import { api } from './api';
import { t } from './i18n.svelte';
import { app, toast } from './store.svelte';

async function check() {
  const token = location.hash.match(/^#\/verify\/([0-9a-f]{64})/)?.[1];
  if (!token) return;
  history.replaceState(null, '', location.pathname);
  try {
    await api('POST', '/auth/verify', { token });
    if (app.me) app.me = { ...app.me, email_verified: true };
    toast(t('verify_done'));
  } catch (e) {
    toast(e instanceof Error ? e.message : t('common_error'));
  }
}

export async function resendVerification(lang: string) {
  try {
    await api('POST', '/users/@me/verify', { lang });
    toast(t('verify_sent'));
  } catch (e) {
    toast(e instanceof Error ? e.message : t('common_error'));
  }
}

window.addEventListener('hashchange', check);
setTimeout(check, 0);
