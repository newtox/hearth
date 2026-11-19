<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { app, type Dm } from '../lib/store.svelte';
  import { canShareScreen, joinVoice, leaveVoice, toggleMute, toggleShare, voice } from '../lib/voice.svelte';
  import { memberMenu, openMenu } from '../lib/menus';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import StreamArea from './StreamArea.svelte';

  let { dm, compact = false }: { dm: Dm; compact?: boolean } = $props();

  const here = $derived(voice.channelId === dm.id);
  const people = $derived(dm.voice.flatMap((id) => dm.recipients.find((r) => r.id === id) ?? []));
</script>

{#if compact}
  {#if !here && app.me?.voice}
    <button class="icon" class:live={dm.voice.length > 0} onclick={() => joinVoice(dm)} disabled={voice.connecting} title={dm.voice.length ? t('call_bar_join_call') : t('call_bar_call')} aria-label={dm.voice.length ? t('call_bar_join_call') : t('call_bar_call')}>
      <Icon name="phone" />
    </button>
  {/if}
{:else if here || dm.voice.length}
  <div class="call">
    {#if here}<StreamArea compact />{/if}
    <div class="row bar">
      <div class="row people grow">
        {#each people as p (p.id)}
          <button class="person" class:speaking={here && voice.speaking.includes(p.id)} title={p.display_name} onclick={() => (app.profile = p.id)} oncontextmenu={(e) => openMenu(e, memberMenu(p.id, null))}>
            <Avatar id={p.id} name={p.display_name} image={p.avatar} size={34} />
            {#if here && voice.silent.includes(p.id)}<i><Icon name="micOff" size={11} /></i>{/if}
          </button>
        {/each}
        <span class="muted">{here ? t('call_bar_in_call') : t('common_call_progress')}</span>
      </div>
      {#if here}
        <button class="btn small" class:off={voice.muted} onclick={toggleMute} disabled={!voice.canSpeak} aria-label={voice.muted ? t('common_unmute_microphone') : t('common_mute')}><Icon name={voice.muted ? 'micOff' : 'mic'} size={16} /></button>
        {#if canShareScreen}<button class="btn small" class:primary={voice.sharing} onclick={toggleShare} disabled={!voice.canStream} aria-label={voice.sharing ? t('common_stop_sharing') : t('common_share_screen')}><Icon name="screen" size={16} /></button>{/if}
        <button class="btn small danger" onclick={leaveVoice}><Icon name="leave" size={16} />{t('call_bar_hang_up')}</button>
      {:else if app.me?.voice}
        <button class="btn small primary" onclick={() => joinVoice(dm)} disabled={voice.connecting}><Icon name="phone" size={16} />{t('common_join')}</button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .icon {
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .icon:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .icon.live {
    color: var(--ok);
  }
  .call {
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .bar {
    padding: 8px 14px;
    flex-wrap: wrap;
  }
  .people {
    gap: 8px;
  }
  .person {
    position: relative;
    display: inline-flex;
    border-radius: 50%;
    outline: 2px solid transparent;
    outline-offset: 2px;
  }
  .person.speaking {
    outline-color: var(--ok);
  }
  .person i {
    position: absolute;
    right: -4px;
    bottom: -4px;
    padding: 2px;
    border-radius: 50%;
    background: var(--panel);
    color: var(--danger);
    display: flex;
  }
  .btn.off {
    color: var(--danger);
  }
</style>
