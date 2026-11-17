<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import Icon from './Icon.svelte';

  const invoke = (cmd: string, args: Record<string, unknown> = {}) => (window as any).__TAURI_INTERNALS__.invoke(`plugin:window|${cmd}`, { label: 'main', ...args });
  let maximized = $state(false);

  async function sync() {
    try {
      maximized = await invoke('is_maximized');
    } catch {}
  }

  $effect(() => {
    sync();
    window.addEventListener('resize', sync);
    return () => window.removeEventListener('resize', sync);
  });
</script>

<header class="titlebar" data-tauri-drag-region>
  <span class="name" data-tauri-drag-region>Hearth</span>
  <div class="controls">
    <button onclick={() => invoke('minimize')} aria-label={t('titlebar_minimize')} title={t('titlebar_minimize')}><Icon name="minus" size={14} /></button>
    <button onclick={() => invoke('toggle_maximize').then(sync)} aria-label={maximized ? t('titlebar_restore') : t('titlebar_maximize')} title={maximized ? t('titlebar_restore') : t('titlebar_maximize')}><Icon name={maximized ? 'restore' : 'square'} size={12} /></button>
    <button class="close" onclick={() => invoke('close')} aria-label={t('common_close')} title={t('common_close')}><Icon name="close" size={14} /></button>
  </div>
</header>

<style>
  .titlebar {
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--rail);
    user-select: none;
    -webkit-user-select: none;
  }
  .name {
    padding-left: 14px;
    font-size: 0.78em;
    font-weight: 700;
    color: var(--muted);
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .controls button {
    width: 44px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .controls button:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .controls .close:hover {
    background: #e5484d;
    color: #fff;
  }
</style>
