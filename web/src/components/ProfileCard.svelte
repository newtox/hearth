<script lang="ts">
  import { backLayer } from '../lib/back.svelte';
  import { ask, askText } from '../lib/dialog.svelte';
  import { fromEnglish, locale, t } from '../lib/i18n.svelte';
  import { api } from '../lib/api';
  import { app, attempt, can, canModerate, guild, isBlocked, isFriend, openDm, P, topPosition, type Badge, type User, presenceOf } from '../lib/store.svelte';
  import { openBelow } from '../lib/menus';
  import { hasRole, toggleRole } from '../lib/moderation';
  import { badgeLabel } from '../lib/activity';
  import Avatar from './Avatar.svelte';
  import BadgeIcon from './BadgeIcon.svelte';
  import ProgressModal from './ProgressModal.svelte';
  import BadgeSheet from './BadgeSheet.svelte';
  import Icon from './Icon.svelte';
  import { backdrop, hoverHost } from '../lib/still.svelte';

  let { userId }: { userId: string } = $props();

  const g = $derived(guild());
  const member = $derived(g?.members.find((m) => m.id === userId) ?? null);
  const roles = $derived(g && member ? g.roles.filter((r) => member.roles.includes(r.id)).sort((a, b) => b.position - a.position) : []);
  const myTop = $derived(g && app.me ? topPosition(g, app.me.id) : 0);
  const assignable = $derived(g && member && can(P.MANAGE_ROLES, g) ? g.roles.filter((r) => !r.is_default && r.position < myTop).sort((a, b) => b.position - a.position) : []);
  const canAssign = (roleId: string) => assignable.some((r) => r.id === roleId);

  function addRole(e: MouseEvent) {
    if (!g) return;
    openBelow(
      e.currentTarget as HTMLElement,
      assignable.map((r) => ({ label: r.name, color: r.color ?? 'var(--muted)', checked: () => hasRole(g!, userId, r.id), keep: true, action: () => toggleRole(g!, userId, r.id) })),
    );
  }
  let user = $state<User | null>(null);
  let allBadges = $state<Badge[]>([]);
  let progressTrack = $state<string | null>(null);
  let openBadge = $state<Badge | null>(null);
  const live = $derived(app.guilds.flatMap((x) => x.members).find((m) => m.id === userId)?.badges);

  async function toggleBadge(b: Badge) {
    const has = user?.badges?.some((x) => x.id === b.id);
    await attempt(api(has ? 'DELETE' : 'PUT', `/users/${userId}/badges/${b.id}`));
    user = (await attempt(api<User>('GET', `/users/${userId}`))) ?? user;
  }

  async function setSince(day: string) {
    const id = openBadge?.id;
    if (!id) return;
    await attempt(api('PUT', `/users/${userId}/badges/${id}`, { since: day }));
    user = (await attempt(api<User>('GET', `/users/${userId}`))) ?? user;
    openBadge = user?.badges?.find((x) => x.id === id) ?? openBadge;
  }

  let note = $state('');

  async function saveNote() {
    if (!user || note.trim() === (user.note ?? '')) return;
    const res = await attempt(api<{ note: string }>('PUT', `/users/${userId}/note`, { note }));
    if (res) user.note = res.note;
  }

  $effect(() => {
    user = null;
    attempt(api<User>('GET', `/users/${userId}`)).then((u) => {
      user = u ?? null;
      note = u?.note ?? '';
    });
    if (app.me?.is_admin) attempt(api<Badge[]>('GET', '/badges')).then((b) => (allBadges = b ?? []));
  });

  const shown = $derived(member ?? app.dms.flatMap((d) => d.recipients).find((r) => r.id === userId) ?? user ?? { streaming: false });
  const since = new Intl.DateTimeFormat(locale(), { day: 'numeric', month: 'long', year: 'numeric' });

  function close() {
    app.profile = null;
  }

  async function kick() {
    if (g && (await ask({ title: t('common_remove_name', { name: user?.display_name ?? '' }), text: t('common_name_rejoin_new_invite', { name: user?.display_name ?? '' }), confirm: t('common_remove'), danger: true }))) {
      await attempt(api('DELETE', `/guilds/${g.id}/members/${userId}`));
      close();
    }
  }

  async function ban() {
    if (!g) return;
    const reason = await askText({ title: t('common_ban_name', { name: user?.display_name ?? '' }), text: t('common_name_removed_cannot_rejoin_until', { name: user?.display_name ?? '' }), input: { label: t('common_reason_optional') }, confirm: t('common_ban'), danger: true });
    if (reason === null) return;
    await attempt(api('PUT', `/guilds/${g.id}/bans/${userId}`, { reason }));
    close();
  }
  let moving = $state(false);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && !progressTrack && !openBadge && close()} />

<div class="backdrop" use:backLayer={close} onclick={(e) => e.target === e.currentTarget && close()} role="presentation">
  <div class="panel card" data-hover role="dialog" aria-modal="true" aria-label={t('common_profile')}>
    <div class="banner" use:hoverHost={(on) => (moving = on)} style:background-color={user?.accent_color ?? 'var(--panel-2)'} style:background-image={backdrop(user?.banner, moving)}>
      <button class="x" onclick={close} aria-label={t('common_close')}><Icon name="close" /></button>
    </div>
    {#if user}
      <div class="body">
        <div class="face"><Avatar id={user.id} name={user.display_name} image={user.avatar} size={84} status={presenceOf(user)} /></div>
        <h2>{member?.nickname || user.display_name}</h2>
        <p class="muted">@{user.username}{member?.nickname ? t('profile_card_also_known_as_name', { name: user.display_name }) : ''}</p>
        {#if (live ?? user.badges)?.length}
          <div class="badges">
            {#each live ?? user.badges ?? [] as b (b.id)}
              {@const label = badgeLabel(b)}
              <button class="mark" title={label.name} aria-label={label.name} onclick={() => (openBadge = user?.badges?.find((x) => x.id === b.id) ?? b)}><BadgeIcon badge={b} size={26} /></button>
            {/each}
          </div>
        {/if}
        {#if shown.streaming}
          <div class="activity">
            <span class="live"><Icon name="broadcast" size={16} /> {t('common_sharing_their_screen')}</span>
          </div>
        {/if}
        {#if user.bio}<p class="bio">{user.bio}</p>{/if}
        {#if user.id !== app.me?.id}
          <label class="note">
            <span class="muted small">{t('menu_note')} <span class="faint">· {t('menu_note_only_you')}</span></span>
            <textarea rows="1" maxlength="256" bind:value={note} placeholder={t('menu_note_placeholder')} onblur={saveNote} onkeydown={(e) => e.key === 'Enter' && !e.shiftKey && (e.preventDefault(), e.currentTarget.blur())}></textarea>
          </label>
        {/if}
        {#if roles.length || assignable.length}
          <div class="roles">
            {#each roles as r (r.id)}
              <span class="role" style:--c={r.color ?? 'var(--muted)'}>
                {r.name}
                {#if canAssign(r.id)}<button class="drop" onclick={() => toggleRole(g!, userId, r.id)} aria-label={t('profile_card_remove_role', { role: r.name })} title={t('profile_card_remove_role', { role: r.name })}><Icon name="close" size={11} /></button>{/if}
              </span>
            {/each}
            {#if assignable.length}<button class="role add" onclick={addRole} aria-haspopup="menu" aria-label={t('profile_card_add_role')} title={t('profile_card_add_role')}><Icon name="plus" size={12} /></button>{/if}
          </div>
        {/if}
        <p class="muted small">{t('profile_card_joined_date', { date: since.format(new Date(user.created_at!)) })}{member ? t('profile_card_here_since_date', { date: since.format(new Date(member.joined_at)) }) : ''}</p>
        <div class="row actions">
          {#if user.id === app.me?.id}
            <button class="btn" onclick={() => ((app.profile = null), (app.modal = { kind: 'user' }))}><Icon name="edit" size={16} /> {t('common_edit_profile')}</button>
            {#if user.progress}<button class="btn" onclick={() => (progressTrack = user!.progress!.tracks[0]?.track ?? null)}>{t('profile_card_progress')}</button>{/if}
          {:else}
            <button class="btn primary" onclick={() => openDm(user!.id)}><Icon name="mail" size={16} /> {t('profile_card_send_message')}</button>
            {#if isBlocked(user.id)}
              <button class="btn" onclick={() => attempt(api('DELETE', `/blocks/${user!.id}`))}>{t('common_unblock')}</button>
            {:else if isFriend(user.id)}
              <button class="btn" onclick={() => attempt(api('DELETE', `/friends/${user!.id}`))}>{t('common_remove_friend')}</button>
            {:else if app.friends.incoming.some((f) => f.id === user!.id)}
              <button class="btn" onclick={() => attempt(api('PUT', `/friends/${user!.id}`))}>{t('profile_card_accept_request')}</button>
            {:else if app.friends.outgoing.some((f) => f.id === user!.id)}
              <button class="btn" disabled>{t('profile_card_request_sent')}</button>
            {:else}
              <button class="btn" onclick={() => attempt(api('POST', '/friends', { user_id: user!.id }))}><Icon name="userPlus" size={16} /> {t('profile_card_add_friend')}</button>
            {/if}
            {#if !isBlocked(user.id)}<button class="btn danger" onclick={async () => (await ask({ title: t('profile_card_block_name', { name: user!.display_name }), text: t('profile_card_no_longer_able_message_each'), confirm: t('profile_card_block'), danger: true })) && attempt(api('PUT', `/blocks/${user!.id}`))}>{t('profile_card_block')}</button>{/if}
            {#if g && canModerate(g, user.id, P.KICK_MEMBERS)}<button class="btn danger" onclick={kick}>{t('common_remove')}</button>{/if}
            {#if g && canModerate(g, user.id, P.BAN_MEMBERS)}<button class="btn danger" onclick={ban}>{t('common_ban')}</button>{/if}
          {/if}
        </div>
        {#if app.me?.is_admin && allBadges.length}
          <details>
            <summary class="muted">{t('profile_card_grant_badges')}</summary>
            <div class="badges grant">
              {#each allBadges as b (b.id)}
                <button class="badge" class:dim={!user.badges?.some((x) => x.id === b.id)} style:--c={b.color} onclick={() => toggleBadge(b)} aria-pressed={user.badges?.some((x) => x.id === b.id)}><BadgeIcon badge={b} size={16} />{badgeLabel(b).name}</button>
              {/each}
            </div>
          </details>
        {/if}
      </div>
    {:else}
      <div class="body"><p class="muted">{t('profile_card_loading_profile')}</p></div>
    {/if}
  </div>
</div>

{#if openBadge && user}
  <BadgeSheet badge={openBadge} name={user.display_name} onclose={() => (openBadge = null)} onprogress={user.progress ? () => ((progressTrack = openBadge!.track!), (openBadge = null)) : undefined} onsince={app.me?.is_admin && !openBadge.track ? setSince : undefined} />
{/if}
{#if progressTrack && user?.progress}
  <ProgressModal progress={user.progress} track={progressTrack} own={user.id === app.me?.id} name={user.display_name} onclose={() => (progressTrack = null)} />
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(8, 5, 20, 0.6);
  }
  .note {
    display: grid;
    gap: 2px;
  }
  .note .faint {
    opacity: 0.7;
  }
  .note textarea {
    resize: none;
    field-sizing: content;
    min-height: 1.6em;
    padding: 4px 6px;
    margin: 0 -6px;
    border: 1px solid transparent;
    border-radius: var(--radius-s);
    background: none;
    color: var(--ink);
    font: inherit;
    font-size: 0.9em;
  }
  .note textarea:hover,
  .note textarea:focus {
    border-color: var(--line);
    background: var(--panel-2);
    outline: none;
  }
  .card {
    animation: pop-in 0.14s ease-out;
    width: min(400px, 100%);
    overflow: hidden;
  }
  .banner {
    height: 120px;
    background-size: cover;
    background-position: center;
    display: flex;
    justify-content: flex-end;
    align-items: flex-start;
    padding: 8px;
  }
  .x {
    padding: 6px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
  }
  .body {
    padding: 0 20px 20px;
    display: grid;
    gap: 8px;
  }
  .face {
    margin-top: -42px;
    width: fit-content;
    border-radius: 50%;
    border: 5px solid var(--panel);
    background: var(--panel);
    line-height: 0;
  }
  :global([data-theme='pergament']) .face {
    border-radius: 4px;
  }
  .bio {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  .roles {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .role {
    padding: 1px 10px;
    border-radius: 99px;
    border: 1px solid var(--c);
    font-size: 0.85em;
    font-weight: 600;
  }
  .role {
    display: inline-flex;
    align-items: center;
  }
  .drop {
    display: grid;
    place-items: center;
    margin: 0 -5px 0 4px;
    padding: 2px;
    border-radius: 50%;
    color: var(--muted);
  }
  .drop:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .role.add {
    --c: var(--line);
    padding: 2px 8px;
    color: var(--muted);
  }
  .role.add::before {
    display: none;
  }
  .role.add:hover {
    color: var(--ink);
    border-color: var(--muted);
  }
  .role::before {
    content: '';
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c);
    margin-right: 6px;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .mark {
    display: inline-flex;
    cursor: default;
  }
  button.mark {
    cursor: pointer;
    border-radius: 6px;
  }
  .activity {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--danger);
    font-weight: 600;
    font-size: 0.88em;
    white-space: nowrap;
  }
  .badges {
    align-items: center;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 10px 2px 7px;
    border-radius: 99px;
    font-size: 0.85em;
    font-weight: 600;
    border: 1px solid var(--c);
    background: color-mix(in srgb, var(--c) 22%, transparent);
  }
  .badge.dim {
    opacity: 0.45;
    background: none;
    border-style: dashed;
  }
  .grant {
    margin-top: 8px;
  }
  summary {
    cursor: pointer;
    font-size: 0.88em;
  }

  .small {
    font-size: 0.85em;
  }
  .actions {
    flex-wrap: wrap;
    margin-top: 6px;
  }
</style>
