<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { app, dmTitle, select } from '../lib/store.svelte';
  import { startRing, stopRing } from '../lib/sounds';
  import { joinVoice } from '../lib/voice.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  const dm = $derived(app.dms.find((d) => d.id === app.ring?.channelId) ?? null);
  const caller = $derived(dm?.recipients.find((r) => r.id === app.ring?.userId) ?? null);

  $effect(() => {
    startRing();
    return stopRing;
  });

  function accept() {
    if (!dm) return;
    select(null, dm.id);
    joinVoice(dm);
    app.ring = null;
  }
</script>

{#if dm && caller}
  <div class="panel ring" role="alertdialog" aria-label={t('ring_banner_incoming_call')}>
    <Avatar id={caller.id} name={caller.display_name} image={caller.avatar} size={44} />
    <div class="grow">
      <strong class="ellipsis">{caller.display_name}</strong>
      <p class="muted ellipsis">{dm.kind === 'group' ? t('ring_banner_calling_name', { name: dmTitle(dm) }) : t('ring_banner_calling')}</p>
    </div>
    {#if app.me?.voice}<button class="btn primary" onclick={accept}><Icon name="phone" size={16} />{t('common_accept')}</button>{/if}
    <button class="btn danger" onclick={() => (app.ring = null)}>{t('common_decline')}</button>
  </div>
{/if}

<style>
  .ring {
    position: fixed;
    top: 18px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 90;
    width: min(460px, calc(100vw - 24px));
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border-color: var(--ok);
  }
  .grow {
    display: grid;
  }
</style>
