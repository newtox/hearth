<script lang="ts">
  import { t } from './lib/i18n.svelte';
  import { app, channel, guild } from './lib/store.svelte';
  import VoiceStage from './components/VoiceStage.svelte';
  import FriendsPane from './components/FriendsPane.svelte';
  import GroupModal from './components/GroupModal.svelte';
  import RingBanner from './components/RingBanner.svelte';
  import Auth from './components/Auth.svelte';
  import Dock from './components/Dock.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Chat from './components/Chat.svelte';
  import Members from './components/Members.svelte';
  import ProfileCard from './components/ProfileCard.svelte';
  import CreateJoin from './components/CreateJoin.svelte';
  import ChannelEditor from './components/ChannelEditor.svelte';
  import GuildSettings from './components/GuildSettings.svelte';
  import UserSettings from './components/UserSettings.svelte';
</script>

{#if !app.authed}
  <Auth />
{:else if !app.ready}
  <div class="loading"><p class="muted">{t('Verbinde …')}</p></div>
{:else}
  <div class="shell">
    <div class="window">
      <Dock />
      <div class="cols" data-pane={app.pane}>
        <Sidebar />
        {#if channel()?.kind === 'voice'}<VoiceStage />{:else if !app.guildId && !app.channelId}<FriendsPane />{:else}<Chat />{/if}
        {#if guild()}<Members />{/if}
      </div>
    </div>
  </div>
  {#if app.modal?.kind === 'create'}<CreateJoin code={app.modal.code} />{/if}
  {#if app.modal?.kind === 'channel'}<ChannelEditor modal={app.modal} />{/if}
  {#if app.modal?.kind === 'guild' && guild()}<GuildSettings tab={app.modal.tab} />{/if}
  {#if app.modal?.kind === 'user'}<UserSettings />{/if}
  {#if app.modal?.kind === 'groupchat'}<GroupModal dm={app.modal.dm} />{/if}
  {#if app.ring}<RingBanner />{/if}
  {#if app.profile}<ProfileCard userId={app.profile} />{/if}
{/if}

{#if app.toast}<div class="toast panel" role="alert">{app.toast}</div>{/if}

<style>
  .loading {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .shell {
    height: 100dvh;
    padding: var(--gap);
  }
  .window {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--gap);
  }
  :global([data-style='modern']) .window {
    overflow: hidden;
    background: var(--panel);
  }
  :global([data-style='modern']) .window :global(.panel) {
    border-radius: 0;
    border: 0;
    box-shadow: none;
  }
  :global([data-style='modern']) .cols > :global(* + *) {
    border-left: 1px solid var(--line);
  }
  @media (min-width: 861px) {
    :global([data-style='modern']) .window {
      flex-direction: row;
    }
  }
  .cols {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: var(--gap);
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    z-index: 100;
    padding: 10px 18px;
    max-width: calc(100vw - 32px);
    border-color: var(--danger);
  }
  @media (max-width: 860px) {
    .cols > :global(*) {
      display: none;
    }
    .cols[data-pane='side'] > :global(.pane-side),
    .cols[data-pane='chat'] > :global(.pane-chat),
    .cols[data-pane='members'] > :global(.pane-members) {
      display: flex;
      flex: 1;
      width: auto;
    }
  }
</style>
