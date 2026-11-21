<script lang="ts">
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { friendMenu, openBelow, openMenu } from '../lib/menus';
  import { app, attempt, openDm, toast, type User, presenceOf } from '../lib/store.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  type Tab = 'online' | 'all' | 'pending' | 'blocked' | 'add';

  let tab = $state<Tab>('online');
  let name = $state('');
  let query = $state('');

  const sorted = $derived([...app.friends.friends].sort((a, b) => Number(!!b.online) - Number(!!a.online) || a.display_name.localeCompare(b.display_name)));
  const pending = $derived(app.friends.incoming.length + app.friends.outgoing.length);
  const match = (u: User) => {
    const q = query.trim().toLowerCase();
    return !q || u.display_name.toLowerCase().includes(q) || u.username.toLowerCase().includes(q);
  };
  const list = $derived((tab === 'online' ? sorted.filter((u) => u.online) : tab === 'all' ? sorted : tab === 'blocked' ? app.friends.blocked : []).filter(match));
  const tabs = $derived<{ id: Tab; label: string; count?: number }[]>([
    { id: 'online', label: t('common_online') },
    { id: 'all', label: t('friends_pane_tab_all') },
    { id: 'pending', label: t('friends_pane_tab_pending'), count: app.friends.incoming.length },
    { id: 'blocked', label: t('friends_pane_blocked') },
  ]);

  function choose(next: Tab) {
    tab = next;
    query = '';
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    if (await attempt(api('POST', '/friends', { username: name }))) {
      toast(t('friends_pane_request_sent_name', { name }));
      name = '';
    }
  }
</script>

<section class="panel pane-chat">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'side')} aria-label={t('common_channel_list')}><Icon name="menu" /></button>
    <Icon name="users" />
    <h3>{t('common_friends')}</h3>
    <nav class="tabs row" aria-label={t('common_friends')}>
      {#each tabs as x (x.id)}
        <button class="tab" class:on={tab === x.id} aria-current={tab === x.id ? 'page' : undefined} onclick={() => choose(x.id)}>
          {x.label}{#if x.count}<span class="count">{x.count}</span>{/if}
        </button>
      {/each}
      <button class="tab add" class:on={tab === 'add'} aria-current={tab === 'add' ? 'page' : undefined} onclick={() => choose('add')}>{t('friends_pane_add_friend')}</button>
    </nav>
  </header>
  <div class="body scroll">
    {#key tab}
      <div class="view">
        {#if tab === 'add'}
          <h4 class="lead">{t('friends_pane_add_friend')}</h4>
          <p class="muted">{t('friends_pane_add_friend_hint')}</p>
          <form class="row" onsubmit={add}>
            <input class="input" bind:value={name} placeholder={t('friends_pane_username_friend')} autocapitalize="off" required />
            <button class="btn primary"><Icon name="userPlus" size={16} />{t('friends_pane_send_request')}</button>
          </form>
        {:else if tab === 'pending'}
          {#if app.friends.incoming.length}
            <h4>{t('friends_pane_incoming_requests')} ({app.friends.incoming.length})</h4>
            {#each app.friends.incoming as u (u.id)}
              <div class="row line" role="presentation" oncontextmenu={(e) => openMenu(e, friendMenu(u.id))}>
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
            <h4>{t('friends_pane_sent_requests')} ({app.friends.outgoing.length})</h4>
            {#each app.friends.outgoing as u (u.id)}
              <div class="row line" role="presentation" oncontextmenu={(e) => openMenu(e, friendMenu(u.id))}>
                <button class="row who grow" onclick={() => (app.profile = u.id)}>
                  <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
                  <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
                </button>
                <button class="btn small" onclick={() => attempt(api('DELETE', `/friends/${u.id}`))}>{t('friends_pane_cancel')}</button>
              </div>
            {/each}
          {/if}
          {#if !pending}<p class="muted empty">{t('friends_pane_no_pending')}</p>{/if}
        {:else}
          <input class="input search" type="search" bind:value={query} placeholder={t('friends_pane_search')} aria-label={t('friends_pane_search')} />
          <h4>{tab === 'online' ? t('friends_pane_online_count', { n: list.length }) : tab === 'blocked' ? `${t('friends_pane_blocked')} (${list.length})` : t('friends_pane_all_count', { n: list.length })}</h4>
          {#each list as u (u.id)}
            <div class="row line" class:off={tab !== 'blocked' && !u.online} role="presentation" oncontextmenu={(e) => openMenu(e, friendMenu(u.id))}>
              <button class="row who grow" onclick={() => (app.profile = u.id)}>
                <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} status={tab === 'blocked' ? undefined : presenceOf(u)} />
                <span class="names">
                  <strong class="ellipsis">{u.display_name}</strong>
                  <small class="muted ellipsis">{tab === 'blocked' ? `@${u.username}` : u.online ? t('common_online') : t('common_offline')}</small>
                </span>
              </button>
              {#if tab === 'blocked'}
                <button class="btn small" onclick={() => attempt(api('DELETE', `/blocks/${u.id}`))}>{t('common_unblock')}</button>
              {:else}
                <button class="icon" onclick={() => openDm(u.id)} title={t('friends_pane_message')} aria-label={t('friends_pane_message')}><Icon name="message" size={17} /></button>
                <button class="icon" onclick={(e) => openBelow(e.currentTarget, friendMenu(u.id))} title={t('friends_pane_more')} aria-label={t('friends_pane_more')} aria-haspopup="menu"><Icon name="more" size={17} /></button>
              {/if}
            </div>
          {:else}
            <p class="muted empty">
              {#if query.trim()}{t('friends_pane_no_match')}{:else if tab === 'online'}{t('friends_pane_nobody_online')}{:else if tab === 'blocked'}{t('friends_pane_nobody_blocked')}{:else}{t('friends_pane_no_friends_yet_send_request')}{/if}
            </p>
          {/each}
        {/if}
      </div>
    {/key}
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
    padding: 10px 16px;
    border-bottom: 1px solid var(--line);
    min-height: 54px;
    gap: 10px;
  }
  header h3 {
    margin: 0;
    padding-right: 10px;
    border-right: 1px solid var(--line);
  }
  .tabs {
    gap: 4px;
    min-width: 0;
    flex-wrap: wrap;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: var(--radius-s);
    color: var(--muted);
    font-weight: 600;
    white-space: nowrap;
    transition: background 0.12s, color 0.12s;
  }
  .tab:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .tab.on {
    color: var(--ink);
    background: var(--panel-2);
  }
  .tab.add {
    margin-left: 4px;
    color: #fff;
    background: var(--accent);
  }
  .tab.add.on {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .count {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 99px;
    background: var(--danger);
    color: #fff;
    font-size: 0.75em;
    line-height: 18px;
    text-align: center;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 18px;
    max-width: 760px;
    width: 100%;
  }
  .view {
    animation: rise-in 0.16s ease-out;
  }
  .search {
    margin-bottom: 4px;
  }
  h4 {
    margin: 18px 0 6px;
    font-size: 0.88em;
    color: var(--muted);
  }
  .lead {
    margin-top: 0;
    font-size: 1em;
    color: var(--ink);
  }
  .empty {
    margin-top: 18px;
  }
  .line {
    padding: 8px 6px;
    border-bottom: 1px solid var(--line);
    border-radius: var(--radius-s);
  }
  .line:hover {
    background: var(--panel-2);
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
    padding: 7px;
    border-radius: 99px;
    color: var(--muted);
    background: var(--panel);
  }
  .icon:hover {
    color: var(--ink);
    background: var(--bg, var(--panel));
  }
  .mobile {
    display: none;
    background: none;
  }
  @media (max-width: 860px) {
    .mobile {
      display: block;
    }
    header h3 {
      display: none;
    }
    form {
      flex-wrap: wrap;
    }
  }
</style>
