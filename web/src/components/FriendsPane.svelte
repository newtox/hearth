<script lang="ts">
  import { ask } from '../lib/dialog.svelte';
  import { activityText } from '../lib/activity';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { app, attempt, openDm, toast, type User, presenceOf } from '../lib/store.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  let name = $state('');
  const sorted = $derived([...app.friends.friends].sort((a, b) => Number(!!b.online) - Number(!!a.online) || a.display_name.localeCompare(b.display_name)));

  async function add(e: SubmitEvent) {
    e.preventDefault();
    if (await attempt(api('POST', '/friends', { username: name }))) {
      toast(t('friends_pane_request_sent_name', { name }));
      name = '';
    }
  }

  async function remove(u: User) {
    if (await ask({ title: t('common_remove_friend'), text: t('friends_pane_really_remove_name_as_friend', { name: u.display_name }), confirm: t('common_remove'), danger: true })) attempt(api('DELETE', `/friends/${u.id}`));
  }
</script>

<section class="panel pane-chat">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'side')} aria-label={t('common_channel_list')}><Icon name="menu" /></button>
    <Icon name="users" />
    <h3 class="grow">{t('common_friends')}</h3>
  </header>
  <div class="body scroll">
    <form class="row" onsubmit={add}>
      <input class="input" bind:value={name} placeholder={t('friends_pane_username_friend')} autocapitalize="off" required />
      <button class="btn primary"><Icon name="userPlus" size={16} />{t('friends_pane_send_request')}</button>
    </form>

    {#if app.friends.incoming.length}
      <h4>{t('friends_pane_incoming_requests')}</h4>
      {#each app.friends.incoming as u (u.id)}
        <div class="row line">
          <button class="row who grow" onclick={() => (app.profile = u.id)}>
            <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
            <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
          </button>
          <button class="btn small primary" onclick={() => attempt(api('PUT', `/friends/${u.id}`))}>{t('common_accept')}</button>
          <button class="btn small" onclick={() => attempt(api('DELETE', `/friends/${u.id}`))}>{t('common_decline')}</button>
        </div>
      {/each}
    {/if}

    {#if app.friends.outgoing.length}
      <h4>{t('friends_pane_sent_requests')}</h4>
      {#each app.friends.outgoing as u (u.id)}
        <div class="row line">
          <button class="row who grow" onclick={() => (app.profile = u.id)}>
            <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
            <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
          </button>
          <button class="btn small" onclick={() => attempt(api('DELETE', `/friends/${u.id}`))}>{t('friends_pane_cancel')}</button>
        </div>
      {/each}
    {/if}

    {#if app.friends.blocked.length}
      <h4>{t('friends_pane_blocked')}</h4>
      {#each app.friends.blocked as u (u.id)}
        <div class="row line">
          <button class="row who grow" onclick={() => (app.profile = u.id)}>
            <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
            <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
          </button>
          <button class="btn small" onclick={() => attempt(api('DELETE', `/blocks/${u.id}`))}>{t('common_unblock')}</button>
        </div>
      {/each}
    {/if}

    <h4>{t('friends_pane_all_friends')} ({sorted.length})</h4>
    {#each sorted as u (u.id)}
      <div class="row line" class:off={!u.online}>
        <button class="row who grow" onclick={() => (app.profile = u.id)}>
          <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} status={presenceOf(u)} />
          <span class="names">
            <strong class="ellipsis">{u.display_name}</strong>
            <small class="muted ellipsis">{u.activity ? activityText(u.activity) : u.online ? t('common_online') : t('common_offline')}</small>
          </span>
        </button>
        <button class="btn small" onclick={() => openDm(u.id)}><Icon name="mail" size={15} />{t('friends_pane_message')}</button>
        <button class="icon" onclick={() => remove(u)} aria-label={t('common_remove_name', { name: u.display_name })}><Icon name="close" size={16} /></button>
      </div>
    {:else}
      <p class="muted">{t('friends_pane_no_friends_yet_send_request')}</p>
    {/each}
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
  .body {
    flex: 1;
    min-height: 0;
    padding: 18px;
    max-width: 760px;
    width: 100%;
  }
  h4 {
    margin: 22px 0 6px;
    font-size: 0.88em;
    color: var(--muted);
  }
  .line {
    padding: 8px 6px;
    border-bottom: 1px solid var(--line);
  }
  .line.off .who {
    opacity: 0.6;
  }
  .who {
    min-width: 0;
    text-align: left;
  }
  .names {
    display: grid;
    min-width: 0;
    line-height: 1.3;
  }
  .icon {
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .icon:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .mobile {
    display: none;
  }
  @media (max-width: 860px) {
    .mobile {
      display: block;
    }
    form {
      flex-wrap: wrap;
    }
  }
</style>
