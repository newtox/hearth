<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { app, guild, memberColor, type Member, presenceOf } from '../lib/store.svelte';
  import { activityText } from '../lib/activity';
  import { w } from '../lib/theme.svelte';
  import { memberMenu, openMenu } from '../lib/menus';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  const g = $derived(guild()!);
  const groups = $derived.by(() => {
    const out: { name: string; members: Member[] }[] = [];
    const rest = new Set(g.members.filter((m) => m.online));
    for (const role of g.roles.filter((r) => r.hoist)) {
      const members = [...rest].filter((m) => m.roles.includes(role.id));
      members.forEach((m) => rest.delete(m));
      if (members.length) out.push({ name: role.name, members });
    }
    if (rest.size) out.push({ name: w('online'), members: [...rest] });
    const offline = g.members.filter((m) => !m.online);
    if (offline.length) out.push({ name: w('offline'), members: offline });
    return out;
  });
</script>

<aside class="panel pane-members">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'chat')} aria-label={t('Zurück zum Chat')}><Icon name="close" /></button>
    <h3>{w('members')}</h3>
    <span class="muted">{g.members.length}</span>
  </header>
  <div class="list scroll">
    {#each groups as group (group.name)}
      <div class="group">{group.name}, {group.members.length}</div>
      {#each group.members as m (m.id)}
        <button class="member row" class:off={!m.online} onclick={() => (app.profile = m.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.id, g))}>
          <Avatar id={m.id} name={m.display_name} image={m.avatar} size={30} status={presenceOf(m)} />
          <span class="who">
            <span class="ellipsis" style:color={memberColor(g, m.id)}>{m.nickname || m.display_name}</span>
            {#if m.activity}<small class="ellipsis muted">{activityText(m.activity)}</small>{/if}
          </span>
          {#if m.streaming}<span class="live" title={t('Teilt den Bildschirm')}><Icon name="broadcast" size={15} /></span>{/if}
          {#if m.id === g.owner_id}<span class="crown" title={t('Besitzer')}><Icon name="crown" size={14} /></span>{/if}
        </button>
      {/each}
    {/each}
  </div>
</aside>

<style>
  aside {
    width: 230px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    padding: 14px 14px 10px;
    border-bottom: 1px solid var(--line);
  }
  .mobile {
    display: none;
    padding: 4px;
  }
  .list {
    flex: 1;
    padding: 8px;
  }
  .group {
    padding: 12px 6px 4px;
    font-size: 0.85em;
    font-weight: 700;
    color: var(--muted);
    font-family: var(--font-display);
  }
  .member {
    width: 100%;
    padding: 5px 6px;
    border-radius: var(--radius-s);
    text-align: left;
  }
  .who {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .who small {
    font-size: 0.8em;
  }
  .live {
    color: var(--danger);
    margin-left: auto;
  }
  .member:hover {
    background: var(--panel-2);
  }
  .member.off {
    opacity: 0.55;
  }
  .crown {
    color: var(--accent);
    margin-left: auto;
  }
  @media (max-width: 1100px) {
    aside {
      width: 190px;
    }
  }
  @media (max-width: 860px) {
    .mobile {
      display: block;
    }
  }
</style>
