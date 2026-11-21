<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { app, attempt, can, dmTitle, guild, partner, select, P, type Channel, presenceOf } from '../lib/store.svelte';
  import { api, asset } from '../lib/api';
  import { w } from '../lib/theme.svelte';
  import { leaveVoice, toggleMute, voice, voiceMembers } from '../lib/voice.svelte';
  import { channelMenu, friendMenu, groupMenu, guildMenu, memberMenu, openBelow, openMenu } from '../lib/menus';
  import Avatar from './Avatar.svelte';
  import { sortable, type Place } from '../lib/drag';
  import Icon from './Icon.svelte';
  import StillImg from './StillImg.svelte';

  const g = $derived(guild());
  const manage = $derived(can(P.MANAGE_CHANNELS, g));
  const tree = $derived.by(() => {
    if (!g) return [];
    const text = g.channels.filter((c) => c.kind !== 'category');
    const groups: { category: Channel | null; items: Channel[] }[] = [{ category: null, items: text.filter((c) => !g.channels.some((p) => p.id === c.parent_id)) }];
    for (const cat of g.channels.filter((c) => c.kind === 'category')) groups.push({ category: cat, items: text.filter((c) => c.parent_id === cat.id) });
    return groups.filter((x) => x.items.length || (x.category && manage));
  });
  function sortPlace(dragId: string, targetId: string, ratio: number): Place | null {
    if (!g || dragId === targetId) return null;
    const drag = g.channels.find((c) => c.id === dragId);
    const target = g.channels.find((c) => c.id === targetId);
    if (!drag || !target) return null;
    if (drag.kind === 'category') return target.kind === 'category' ? (ratio < 0.5 ? 'before' : 'after') : null;
    if (target.kind === 'category') return 'inside';
    return ratio < 0.5 ? 'before' : 'after';
  }

  async function sortDrop(dragId: string, targetId: string, place: Place) {
    if (!g) return;
    const all = g.channels;
    const cats = all.filter((c) => c.kind === 'category');
    const groups = [{ id: null as string | null, items: all.filter((c) => c.kind !== 'category' && !cats.some((x) => x.id === c.parent_id)) }, ...cats.map((cat) => ({ id: cat.id as string | null, items: all.filter((c) => c.parent_id === cat.id && c.kind !== 'category') }))];
    const drag = all.find((c) => c.id === dragId)!;
    if (drag.kind === 'category') {
      const from = groups.findIndex((x) => x.id === dragId);
      const [moved] = groups.splice(from, 1);
      const to = groups.findIndex((x) => x.id === targetId);
      groups.splice(place === 'before' ? to : to + 1, 0, moved);
    } else {
      for (const grp of groups) grp.items = grp.items.filter((c) => c.id !== dragId);
      if (place === 'inside') groups.find((x) => x.id === targetId)?.items.unshift(drag);
      else {
        const grp = groups.find((x) => x.items.some((c) => c.id === targetId))!;
        const at = grp.items.findIndex((c) => c.id === targetId);
        grp.items.splice(place === 'before' ? at : at + 1, 0, drag);
      }
    }
    const order: { id: string; parent_id: string | null }[] = [];
    for (const grp of groups) {
      if (grp.id) order.push({ id: grp.id, parent_id: null });
      for (const c of grp.items) order.push({ id: c.id, parent_id: grp.id });
    }
    const before = all;
    g.channels = order.map((o, i) => ({ ...all.find((c) => c.id === o.id)!, parent_id: o.parent_id, position: i }));
    if (!(await attempt(api('PUT', `/guilds/${g.id}/channels`, order)))) g.channels = before;
  }

  let menuFromHeader = $state(false);
  $effect(() => {
    if (!app.menu) menuFromHeader = false;
  });
  const voiceName = $derived(app.guilds.flatMap((x) => x.channels).find((c) => c.id === voice.channelId)?.name ?? '');
</script>

<aside class="panel pane-side">
  {#if g}
    <header>
      <button class="title row" class:open={!!app.menu && menuFromHeader} aria-haspopup="menu" onclick={(e) => { menuFromHeader = true; openBelow(e.currentTarget, guildMenu(g)); }} oncontextmenu={(e) => openMenu(e, guildMenu(g))}>
        <span class="gicon"><Avatar id={g.id} name={g.name} image={g.icon} size={40} square /></span>
        <span class="gname">
          <h3 class="grow ellipsis">{g.name}</h3>
          <small class="muted">{g.members.length} {w('members')}</small>
        </span>
        <Icon name="chevron" size={18} />
      </button>
    </header>
    <div class="list scroll">
      {#each tree as group (group.category?.id ?? 'none')}
        <div class="group" class:card={!!group.category}>
        {#if group.category}
          <div class="category row" role="presentation" use:sortable={{ id: group.category.id, group: 'channels', label: group.category.name, enabled: manage, place: sortPlace, drop: sortDrop }} oncontextmenu={(e) => openMenu(e, channelMenu(g, group.category!))}>
            <span class="grow ellipsis">{group.category.name}</span>
            {#if manage}
              <button class="icon" onclick={() => (app.modal = { kind: 'channel', channel: group.category })} aria-label={t('common_edit_category')}><Icon name="edit" size={14} /></button>
              <button class="icon" onclick={() => (app.modal = { kind: 'channel', channel: null, parent: group.category!.id })} aria-label={t('common_create_channel')}><Icon name="plus" size={15} /></button>
            {/if}
          </div>
        {/if}
        {#each group.items as c (c.id)}
          <div class="item" class:active={c.id === app.channelId} class:unread={app.unread[c.id]} role="presentation" use:sortable={{ id: c.id, group: 'channels', label: c.name, enabled: manage, place: sortPlace, drop: sortDrop }} oncontextmenu={(e) => openMenu(e, channelMenu(g, c))}>
            <button class="main row" onclick={() => select(g.id, c.id)}>
              <Icon name={c.kind === 'voice' ? 'volume' : 'hash'} size={16} />
              <span class="ellipsis">{c.name}</span>
            </button>
            {#if manage}
              <button class="icon edit" onclick={() => (app.modal = { kind: 'channel', channel: c })} aria-label={t('common_edit_name', { name: c.name })}><Icon name="edit" size={14} /></button>
            {/if}
          </div>
          {#if c.kind === 'voice'}
            {#each voiceMembers(c.id) as m (m.id)}
              <button class="row inside" class:speaking={voice.channelId === c.id && voice.speaking.includes(m.id)} onclick={() => (app.profile = m.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.id, g))}>
                <Avatar id={m.id} name={m.display_name} image={m.avatar} size={20} />
                <span class="ellipsis">{m.nickname || m.display_name}</span>
              </button>
            {/each}
          {/if}
        {/each}
        </div>
      {/each}
    </div>
  {:else}
    <header><h3>{w('dms')}</h3></header>
    <div class="list scroll">
      <div class="item" class:active={!app.channelId}>
        <button class="main row" onclick={() => (select(null), (app.pane = 'chat'))}>
          <Icon name="users" size={18} />
          <span class="grow">{t('common_friends')}</span>
          {#if app.friends.incoming.length}<span class="count">{app.friends.incoming.length}</span>{/if}
        </button>
      </div>
      <div class="category row">
        <span class="grow">{t('sidebar_conversations')}</span>
        <button class="icon" onclick={() => (app.modal = { kind: 'groupchat', dm: null })} title={t('common_new_group')} aria-label={t('common_new_group')}><Icon name="plus" size={15} /></button>
      </div>
      {#each app.dms as d (d.id)}
        {@const p = partner(d)}
        <div class="item" class:active={d.id === app.channelId} class:unread={app.unread[d.id]} role="presentation" oncontextmenu={(e) => openMenu(e, d.kind === 'group' ? groupMenu(d) : friendMenu(p.id, d))}>
          <button class="main row" onclick={() => select(null, d.id)}>
            {#if d.kind === 'group'}
              <span class="groupicon">{#if d.icon}<StillImg src={asset(d.icon)!} alt="" />{:else}<Icon name="users" size={16} />{/if}</span>
            {:else}
              <Avatar id={p.id} name={p.display_name} image={p.avatar} size={30} status={presenceOf(p)} />
            {/if}
            <span class="ellipsis grow">{dmTitle(d)}</span>
            {#if d.voice.length}<span class="incall" title={t('common_call_progress')}><Icon name="phone" size={14} /></span>{/if}
          </button>
        </div>
      {:else}
        <p class="muted empty">{t('sidebar_no_conversations_yet_message_friend', { server: w('server') })}</p>
      {/each}
    </div>
  {/if}
  {#if voice.channelId}
    <div class="voicebar row">
      <span class="live"></span>
      <span class="grow ellipsis">{t('sidebar_connected_name', { name: voiceName })}</span>
      <button class="icon" onclick={toggleMute} aria-label={voice.muted ? t('common_unmute_microphone') : t('common_mute')}><Icon name={voice.muted ? 'micOff' : 'mic'} size={17} /></button>
      <button class="icon" onclick={leaveVoice} aria-label={t('sidebar_leave_voice_channel')}><Icon name="leave" size={17} /></button>
    </div>
  {/if}
</aside>

<style>
  aside {
    width: 250px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    padding: 14px 14px 10px;
    border-bottom: 1px solid var(--line);
  }
  header:has(.title) {
    padding: 6px;
  }
  .title {
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-s);
    text-align: left;
    color: var(--ink);
  }
  .title h3 {
    margin: 0;
  }
  .title :global(svg) {
    flex: none;
    color: var(--muted);
    transition: transform 0.15s;
  }
  .title:hover,
  .title.open {
    background: var(--panel-2);
  }
  .title.open :global(svg) {
    transform: rotate(180deg);
  }
  @media (prefers-reduced-motion: reduce) {
    .title :global(svg) {
      transition: none;
    }
  }
  .list {
    flex: 1;
    padding: 8px;
  }
  .category {
    padding: 14px 6px 4px;
    font-size: 0.85em;
    font-weight: 700;
    color: var(--muted);
    font-family: var(--font-display);
  }
  .item {
    display: flex;
    align-items: center;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .item:hover,
  .item.active {
    background: var(--panel-2);
    color: var(--ink);
  }
  .item.active {
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .item.unread {
    color: var(--ink);
    font-weight: 700;
  }
  :global([data-style='modern']) .item {
    border-radius: 10px;
    transition: background 0.12s ease, color 0.12s ease;
  }
  :global([data-style='modern']) .item.active {
    box-shadow: none;
    background: var(--accent-fill);
    color: var(--accent-ink);
  }
  :global([data-style='modern']) .item.active :global(svg),
  :global([data-style='modern']) .item.active :global(button) {
    color: var(--accent-ink);
  }
  .group,
  .gname {
    display: contents;
  }
  .gicon,
  .gname small {
    display: none;
  }
  :global([data-style='modern']) header {
    border-bottom: 0;
  }
  :global([data-style='modern']) header:has(.title) {
    margin: 8px 8px 2px;
    padding: 0;
    border-radius: 16px;
    background: color-mix(in srgb, var(--accent) 14%, var(--panel-2));
  }
  :global([data-style='modern']) .title {
    gap: 11px;
    padding: 9px 12px 9px 9px;
    border-radius: 16px;
  }
  :global([data-style='modern']) .title:hover,
  :global([data-style='modern']) .title.open {
    background: color-mix(in srgb, var(--accent) 22%, var(--panel-2));
  }
  :global([data-style='modern']) .gicon {
    display: flex;
  }
  :global([data-style='modern']) .gname {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  :global([data-style='modern']) .gname small {
    display: block;
    font-size: 0.8em;
  }
  :global([data-style='modern']) .title h3 {
    font-size: 1.08em;
  }
  :global([data-style='modern']) .group.card {
    display: block;
    margin-bottom: 8px;
    padding: 2px 4px 4px;
    border-radius: 16px;
    background: color-mix(in srgb, var(--panel-2) 42%, var(--panel));
  }
  :global([data-style='modern']) .group.card .category {
    padding: 8px 4px 4px 8px;
  }
  .main {
    flex: 1;
    min-width: 0;
    padding: 7px 8px;
    text-align: left;
  }
  .icon {
    padding: 5px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .icon:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .edit {
    opacity: 0;
    margin-right: 4px;
  }
  .item:hover .edit,
  .edit:focus-visible {
    opacity: 1;
  }
  .count {
    padding: 0 7px;
    border-radius: 99px;
    background: var(--danger);
    color: #fff;
    font-size: 0.8em;
    font-weight: 700;
  }
  .groupicon {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--panel-2);
    border: 1px solid var(--line);
    overflow: hidden;
  }
  .groupicon :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .incall {
    color: var(--ok);
  }
  .inside {
    width: 100%;
    padding: 3px 8px 3px 32px;
    font-size: 0.9em;
    color: var(--muted);
    text-align: left;
  }
  .voicebar {
    padding: 10px 12px;
    border-top: 1px solid var(--line);
    font-size: 0.9em;
    font-weight: 600;
  }
  .live {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--ok);
  }
  .empty {
    padding: 10px;
  }
  @media (hover: none) {
    .edit {
      opacity: 1;
    }
  }
  .inside :global(.avatar) {
    transition: box-shadow 0.08s;
  }
  .inside.speaking :global(.avatar) {
    box-shadow: 0 0 0 2px var(--ok);
  }
  .inside.speaking {
    color: var(--ink);
  }
</style>
