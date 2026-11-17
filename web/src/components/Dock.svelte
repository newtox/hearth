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
    <button class="tab add" onclick={() => (app.modal = { kind: 'create' })} title={t('{server} erstellen oder beitreten', { server: w('server') })} aria-label={t('{server} erstellen oder beitreten', { server: w('server') })}>
      <Icon name="plus" />
    </button>
  </div>
  <button class="tab me" onclick={(e) => openMenu(e, statusMenu())} oncontextmenu={(e) => openMenu(e, statusMenu())} title={t('Status und Einstellungen')} aria-haspopup="menu">
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
    background: var(--rail);
  }
  @media (min-width: 861px) {
    :global([data-style='modern']) .dock {
      flex-direction: column;
      width: 76px;
      padding: 14px 0;
      gap: 12px;
    }
    :global([data-style='modern']) .tabs {
      flex-direction: column;
      align-items: center;
      gap: 12px;
      overflow: hidden auto;
      width: 100%;
      padding: 4px 0;
    }
    :global([data-style='modern']) .label {
      display: none;
    }
    :global([data-style='modern']) .tab {
      width: 48px;
      height: 48px;
      padding: 0;
      justify-content: center;
      border-radius: 50%;
      background: var(--panel-2);
      color: var(--ink);
    }
    :global([data-style='modern']) .tab :global(.avatar) {
      --s: 42px !important;
      border-radius: 50%;
    }
    :global([data-style='modern']) .tab.active {
      background: var(--accent-fill);
    }
    :global([data-style='modern']) .tab.active::before {
      content: '';
      position: absolute;
      left: -14px;
      width: 5px;
      height: 30px;
      border-radius: 0 6px 6px 0;
      background: var(--accent-fill);
    }
    :global([data-style='modern']) .tab.add {
      background: transparent;
      border: 1.5px dashed rgba(255, 255, 255, 0.35);
    }
    :global([data-style='modern']) .pip {
      top: 0;
      right: 0;
      border: 2px solid var(--rail);
      width: 13px;
      height: 13px;
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
