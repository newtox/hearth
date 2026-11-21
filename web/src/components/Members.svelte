<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { app, guild, memberColor, type Member, presenceOf } from '../lib/store.svelte';
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

  let slim = $state(read());
  function read() {
    try {
      return localStorage.getItem('hearth_members_slim') === '1';
    } catch {
      return false;
    }
  }
  function toggle() {
    slim = !slim;
    try {
      localStorage.setItem('hearth_members_slim', slim ? '1' : '0');
    } catch {}
  }
</script>

<aside class="panel pane-members" class:slim>
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'chat')} aria-label={t('members_back_chat')}><Icon name="close" /></button>
    <h3 class="label">{w('members')}</h3>
    <span class="muted label grow">{g.members.length}</span>
    <button class="icon fold" onclick={toggle} aria-label={slim ? t('members_expand') : t('members_collapse')} title={slim ? t('members_expand') : t('members_collapse')} aria-expanded={!slim}><Icon name="chevron" size={16} /></button>
  </header>
  <div class="list scroll">
    {#each groups as group (group.name)}
      <div class="group"><span class="label">{group.name}, {group.members.length}</span></div>
      {#each group.members as m (m.id)}
        <button class="member row" class:off={!m.online} onclick={() => (app.profile = m.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.id, g))} title={slim ? m.nickname || m.display_name : undefined}>
          <Avatar id={m.id} name={m.display_name} image={m.avatar} size={30} status={presenceOf(m)} />
          <span class="who">
            <span class="ellipsis" style:color={memberColor(g, m.id)}>{m.nickname || m.display_name}</span>
          </span>
          {#if m.streaming}<span class="live" title={t('common_sharing_their_screen')}><Icon name="broadcast" size={15} /></span>{/if}
          {#if m.id === g.owner_id}<span class="crown" title={t('common_owner')}><Icon name="crown" size={14} /></span>{/if}
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
  .fold {
    padding: 4px;
    border-radius: 999px;
    color: var(--muted);
    transform: rotate(-90deg);
    transition: transform 0.18s ease;
  }
  .fold:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  aside {
    transition: width 0.18s ease;
  }
  @media (min-width: 861px) {
    .slim {
      width: 66px;
    }
    .slim header {
      justify-content: center;
      padding-left: 6px;
      padding-right: 6px;
    }
    .slim .fold {
      transform: rotate(90deg);
    }
    .slim .label,
    .slim .who,
    .slim .live,
    .slim .crown {
      display: none;
    }
    .slim .list {
      padding: 6px;
    }
    .slim .group {
      height: 1px;
      margin: 10px 8px 8px;
      padding: 0;
      background: var(--line);
    }
    .slim .member {
      justify-content: center;
      padding: 5px 0;
    }
  }
  :global([data-style='modern']) header {
    border-bottom: 0;
  }
  :global([data-style='modern']) .member {
    border-radius: 12px;
  }
  @media (prefers-reduced-motion: reduce) {
    aside,
    .fold {
      transition: none;
    }
  }
  @media (max-width: 860px) {
    .mobile {
      display: block;
    }
    .fold {
      display: none;
    }
  }
</style>
