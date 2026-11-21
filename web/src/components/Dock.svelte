<script lang="ts">
  import { guildMenu, openMenu, statusMenu } from '../lib/menus';
  import { t } from '../lib/i18n.svelte';
  import { app, dmUnread, guildUnread, select } from '../lib/store.svelte';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  function open(id: string | null) {
    select(id);
    app.pane = app.channelId ? 'chat' : 'side';
  }
</script>

<nav class="dock panel" aria-label={w('servers')}>
  <button class="tab" class:active={!app.guildId} onclick={() => open(null)} title={w('dms')}>
    <Icon name="mail" />
    <span class="label">{w('dms')}</span>
    {#if dmUnread()}<i class="pip"></i>{/if}
  </button>
  <div class="tabs">
    {#each app.guilds as g (g.id)}
      <button class="tab banner" class:active={app.guildId === g.id} onclick={() => open(g.id)} oncontextmenu={(e) => openMenu(e, guildMenu(g))} title={g.name}>
        <Avatar id={g.id} name={g.name} image={g.icon} size={26} square />
        <span class="label ellipsis">{g.name}</span>
        {#if guildUnread(g)}<i class="pip"></i>{/if}
      </button>
    {/each}
    <button class="tab add" onclick={() => (app.modal = { kind: 'create' })} title={t('common_create_join_server', { server: w('server') })} aria-label={t('common_create_join_server', { server: w('server') })}>
      <Icon name="plus" />
    </button>
  </div>
  <button class="tab me" onclick={(e) => openMenu(e, statusMenu())} oncontextmenu={(e) => openMenu(e, statusMenu())} title={t('dock_status_settings')} aria-haspopup="menu">
    <Avatar id={app.me!.id} name={app.me!.display_name} image={app.me!.avatar} size={28} status={app.connected ? (app.me!.status ?? 'online') : 'offline'} />
    <span class="label ellipsis">{app.me!.display_name}</span>
  </button>
</nav>

<style>
  .dock {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px;
    flex: none;
  }
  .tabs {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 6px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 12px 0 8px;
    max-width: 200px;
    border-radius: calc(var(--radius) - 6px);
    font-weight: 600;
    color: var(--muted);
  }
  .tab:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .tab.active {
    background: var(--accent-fill);
    color: var(--accent-ink);
  }
  :global([data-style='modern']) .dock {
    background: transparent;
    padding: 0 0 2px;
    gap: 8px;
  }
  :global([data-style='modern']) .tabs {
    gap: 8px;
    padding: 2px;
  }
  :global([data-style='modern']) .tab {
    height: 42px;
    padding: 0 16px 0 6px;
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 650;
    transition: background 0.15s ease, color 0.15s ease, box-shadow 0.15s ease;
  }
  :global([data-style='modern']) .tab:not(.banner):not(.me) {
    padding: 0 13px;
  }
  :global([data-style='modern']) .tab :global(.avatar) {
    border-radius: 50%;
  }
  :global([data-style='modern']) .tab:hover {
    background: var(--panel-2);
  }
  :global([data-style='modern']) .tab.active {
    background: var(--accent-fill);
    color: var(--accent-ink);
    box-shadow: 0 8px 22px -10px var(--accent-fill);
  }
  :global([data-style='modern']) .tab.add {
    background: transparent;
    border: 1.5px dashed color-mix(in srgb, var(--muted) 55%, transparent);
    color: var(--muted);
  }
  :global([data-style='modern']) .pip {
    top: 1px;
    right: 1px;
    width: 11px;
    height: 11px;
    border: 2px solid var(--rail);
  }
  @media (max-width: 860px) {
    :global([data-style='modern']) .tab.banner,
    :global([data-style='modern']) .tab.me {
      padding: 0 6px;
    }
  }
  .tab.add {
    padding: 0 11px;
    border: 1px dashed var(--line);
  }
  .pip {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--danger);
  }
  :global([data-theme='pergament']) .dock {
    align-items: flex-start;
  }
  :global([data-theme='pergament']) .tab {
    font-family: var(--font-display);
    border-radius: 2px;
  }
  :global([data-theme='pergament']) .tab.banner {
    height: 50px;
    padding-bottom: 10px;
    background: var(--panel-2);
    color: var(--ink);
    clip-path: polygon(0 0, 100% 0, 100% 100%, 50% calc(100% - 9px), 0 100%);
  }
  :global([data-theme='pergament']) .tab.banner.active {
    background: var(--accent);
    color: var(--accent-ink);
  }
  @media (max-width: 860px) {
    .label {
      display: none;
    }
    .tab {
      padding: 0 8px;
    }
  }
</style>
