<script lang="ts">
  import { activityText } from '../lib/activity';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { app, attempt, openDm, toast, type User } from '../lib/store.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  let name = $state('');
  const sorted = $derived([...app.friends.friends].sort((a, b) => Number(!!b.online) - Number(!!a.online) || a.display_name.localeCompare(b.display_name)));

  async function add(e: SubmitEvent) {
    e.preventDefault();
    if (await attempt(api('POST', '/friends', { username: name }))) {
      toast(t('Anfrage an {name} gesendet', { name }));
      name = '';
    }
  }

  function remove(u: User) {
    if (confirm(t('{name} als Freund entfernen?', { name: u.display_name }))) attempt(api('DELETE', `/friends/${u.id}`));
  }
</script>

<section class="panel pane-chat">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'side')} aria-label={t('Kanalliste')}><Icon name="menu" /></button>
    <Icon name="users" />
    <h3 class="grow">{t('Freunde')}</h3>
  </header>
  <div class="body scroll">
    <form class="row" onsubmit={add}>
      <input class="input" bind:value={name} placeholder={t('Benutzername eines Freundes')} autocapitalize="off" required />
      <button class="btn primary"><Icon name="userPlus" size={16} />{t('Anfrage senden')}</button>
    </form>

    {#if app.friends.incoming.length}
      <h4>{t('Eingegangene Anfragen')}</h4>
      {#each app.friends.incoming as u (u.id)}
        <div class="row line">
          <button class="row who grow" onclick={() => (app.profile = u.id)}>
            <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
            <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
          </button>
          <button class="btn small primary" onclick={() => attempt(api('PUT', `/friends/${u.id}`))}>{t('Annehmen')}</button>
          <button class="btn small" onclick={() => attempt(api('DELETE', `/friends/${u.id}`))}>{t('Ablehnen')}</button>
        </div>
      {/each}
    {/if}

    {#if app.friends.outgoing.length}
      <h4>{t('Gesendete Anfragen')}</h4>
      {#each app.friends.outgoing as u (u.id)}
        <div class="row line">
          <button class="row who grow" onclick={() => (app.profile = u.id)}>
            <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
            <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
          </button>
          <button class="btn small" onclick={() => attempt(api('DELETE', `/friends/${u.id}`))}>{t('Zurückziehen')}</button>
        </div>
      {/each}
    {/if}

    {#if app.friends.blocked.length}
      <h4>{t('Blockiert')}</h4>
      {#each app.friends.blocked as u (u.id)}
        <div class="row line">
          <button class="row who grow" onclick={() => (app.profile = u.id)}>
            <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} />
            <span class="names"><strong class="ellipsis">{u.display_name}</strong><small class="muted">@{u.username}</small></span>
          </button>
          <button class="btn small" onclick={() => attempt(api('DELETE', `/blocks/${u.id}`))}>{t('Blockierung aufheben')}</button>
        </div>
      {/each}
    {/if}

    <h4>{t('Alle Freunde')} ({sorted.length})</h4>
    {#each sorted as u (u.id)}
      <div class="row line" class:off={!u.online}>
        <button class="row who grow" onclick={() => (app.profile = u.id)}>
          <Avatar id={u.id} name={u.display_name} image={u.avatar} size={38} online={!!u.online} />
          <span class="names">
            <strong class="ellipsis">{u.display_name}</strong>
            <small class="muted ellipsis">{u.activity ? activityText(u.activity) : u.online ? t('Online') : t('Offline')}</small>
          </span>
        </button>
        <button class="btn small" onclick={() => openDm(u.id)}><Icon name="mail" size={15} />{t('Nachricht')}</button>
        <button class="icon" onclick={() => remove(u)} aria-label={t('{name} entfernen', { name: u.display_name })}><Icon name="close" size={16} /></button>
      </div>
    {:else}
      <p class="muted">{t('Noch keine Freunde. Schick oben eine Anfrage mit dem Benutzernamen.')}</p>
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
