<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { app, channel, guild, type Channel } from '../lib/store.svelte';
  import { memberMenu, openMenu } from '../lib/menus';
  import { canShareScreen, joinVoice, leaveVoice, toggleMute, toggleShare, voice, voiceMembers } from '../lib/voice.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import StreamArea from './StreamArea.svelte';

  const ch = $derived(channel() as Channel);
  const here = $derived(voice.channelId === ch.id);
  const people = $derived(voiceMembers(ch.id));
</script>

<section class="panel pane-chat">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'side')} aria-label={t('common_channel_list')}><Icon name="menu" /></button>
    <Icon name="volume" />
    <h3 class="grow ellipsis">{ch.name}</h3>
    <button class="icon mobile" onclick={() => (app.pane = 'members')} aria-label={t('common_members')}><Icon name="users" /></button>
  </header>

  <div class="stage scroll">
    {#if here}<StreamArea />{/if}
    <div class="people">
      {#each people as m (m.id)}
        <button class="person" class:speaking={here && voice.speaking.includes(m.id)} onclick={() => (app.profile = m.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.id, guild()))}>
          <Avatar id={m.id} name={m.display_name} image={m.avatar} size={72} />
          <span class="ellipsis">{m.nickname || m.display_name}</span>
          {#if here && voice.silent.includes(m.id)}<span class="muted" title={t('voice_stage_muted')}><Icon name="micOff" size={15} /></span>{/if}
        </button>
      {:else}
        <p class="muted">{t('voice_stage_nobody_here_right_now')}</p>
      {/each}
    </div>
  </div>

  <div class="controls row">
    {#if here}
      <button class="btn" class:off={voice.muted} onclick={toggleMute} disabled={!voice.canSpeak}>
        <Icon name={voice.muted ? 'micOff' : 'mic'} />{voice.muted ? t('voice_stage_muted') : t('voice_stage_microphone')}
      </button>
      {#if canShareScreen}
        <button class="btn" class:primary={voice.sharing} onclick={toggleShare} disabled={!voice.canStream}>
          <Icon name="screen" />{voice.sharing ? t('common_stop_sharing') : t('common_share_screen')}
        </button>
      {/if}
      <button class="btn danger" onclick={leaveVoice}><Icon name="leave" />{t('common_leave')}</button>
    {:else if app.me?.voice}
      <button class="btn primary" onclick={() => joinVoice(ch)} disabled={voice.connecting}><Icon name="volume" />{voice.connecting ? t('common_connecting') : t('common_join')}</button>
    {:else}
      <p class="muted">{t('voice_stage_voice_channels_not_set_up')}</p>
    {/if}
  </div>
</section>

<style>
  section {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--line);
    min-height: 54px;
  }
  .mobile {
    display: none;
  }
  .icon {
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .stage {
    flex: 1;
    min-height: 0;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .people {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 18px;
    margin: auto 0;
  }
  .person {
    width: 120px;
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 14px 8px;
    border-radius: var(--radius);
    border: 2px solid transparent;
    background: var(--panel-2);
    text-align: center;
  }
  .person.speaking {
    border-color: var(--ok);
  }
  .person span {
    max-width: 100%;
  }
  .controls {
    justify-content: center;
    flex-wrap: wrap;
    padding: 14px;
    border-top: 1px solid var(--line);
  }
  .btn.off {
    color: var(--danger);
  }
  @media (max-width: 860px) {
    .mobile {
      display: block;
    }
  }
</style>
