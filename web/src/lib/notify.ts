import { isApp } from './api';

type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
const invoke: Invoke | null = isApp ? (window as any).__TAURI_INTERNALS__.invoke : null;

export const notifications = {
  supported: isApp || 'Notification' in window,
  enabled: () => localStorage.getItem('hearth_notify') === 'on',
  async enable() {
    let granted = false;
    if (invoke) {
      granted = (await invoke('plugin:notification|is_permission_granted').catch(() => false)) === true;
      if (!granted) granted = (await invoke('plugin:notification|request_permission').catch(() => 'denied')) === 'granted';
    } else if ('Notification' in window) {
      granted = (Notification.permission === 'granted' ? 'granted' : await Notification.requestPermission()) === 'granted';
    }
    localStorage.setItem('hearth_notify', granted ? 'on' : 'off');
    return granted;
  },
  disable() {
    localStorage.setItem('hearth_notify', 'off');
  },
};

export function notify(title: string, body: string, tag: string, onclick: () => void) {
  if (!notifications.enabled()) return;
  if (invoke) {
    invoke('plugin:notification|notify', { options: { title, body } }).catch(() => {});
    return;
  }
  if (!('Notification' in window) || Notification.permission !== 'granted') return;
  const n = new Notification(title, { body, tag });
  n.onclick = () => {
    window.focus();
    onclick();
    n.close();
  };
}
