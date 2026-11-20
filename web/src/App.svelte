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
  import ContextMenu from './components/ContextMenu.svelte';
  import ModView from './components/ModView.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import Dialog from './components/Dialog.svelte';
  import { resendVerification } from './lib/verify';
  import { lang } from './lib/i18n.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import { isApp } from './lib/api';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import { watchUpdates } from './lib/update.svelte';

  watchUpdates();

  const framed = isApp && !/android|iphone|ipad/i.test(navigator.userAgent);
</script>

{#if framed}<TitleBar />{/if}
<div class="frame" class:framed onscroll={(e) => (e.currentTarget.scrollTop = 0)}>

{#if !app.authed}
  <Auth />
{:else if !app.ready}
  <div class="loading"><p class="muted">{t('common_connecting')}</p></div>
{:else}
  <div class="shell">
    {#if app.me?.email_verified === false}
      <div class="verify panel row" role="status">
        <span class="grow">{t('verify_banner', { email: app.me.email ?? '' })}</span>
        <button class="btn small" onclick={() => resendVerification(lang)}>{t('verify_resend')}</button>
      </div>
    {/if}
    <UpdateBanner />
    <div class="window">
      <Dock />
      <div class="cols" data-pane={app.pane}>
        <Sidebar />
        {#if channel()?.kind === 'voice'}<VoiceStage />{:else if !app.guildId && !app.channelId}<FriendsPane />{:else}<Chat />{/if}
        {#if app.side && channel() && channel()?.kind !== 'voice'}<SidePanel />{:else if guild()}<Members />{/if}
      </div>
    </div>
  </div>
  {#if app.modal?.kind === 'create'}<CreateJoin code={app.modal.code} />{/if}
  {#if app.modal?.kind === 'channel'}<ChannelEditor modal={app.modal} />{/if}
  {#if app.modal?.kind === 'guild' && guild()}<GuildSettings tab={app.modal.tab} />{/if}
  {#if app.modal?.kind === 'user'}<UserSettings />{/if}
  {#if app.modal?.kind === 'groupchat'}<GroupModal dm={app.modal.dm} />{/if}
  {#if app.modal?.kind === 'mod' && guild()}<ModView userId={app.modal.userId} />{/if}
  <ContextMenu />
  {#if app.ring}<RingBanner />{/if}
  {#if app.profile}<ProfileCard userId={app.profile} />{/if}
{/if}

<Dialog />

{#if app.toast}<div class="toast panel" role="alert">{app.toast}</div>{/if}
</div>

<style>
  .loading {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .frame {
    height: 100%;
  }
  .frame.framed {
    height: calc(100% - 30px);
    position: relative;
    transform: translateZ(0);
    overflow: hidden;
  }
  .shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: max(var(--gap), env(safe-area-inset-top)) max(var(--gap), env(safe-area-inset-right)) max(var(--gap), env(safe-area-inset-bottom)) max(var(--gap), env(safe-area-inset-left));
  }
  .window {
    flex: 1;
    min-height: 0;
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
  .verify {
    gap: 10px;
    padding: 8px 12px;
    margin-bottom: var(--gap);
    font-size: 0.9em;
    border-left: 3px solid var(--danger);
  }
</style>
